// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {SunmintCertificates} from "../src/SunmintCertificates.sol";
import {SunmintMarket} from "../src/SunmintMarket.sol";
import {TestUSD} from "../src/TestUSD.sol";

contract SunmintTest is Test {
    SunmintCertificates certs;
    SunmintMarket market;
    TestUSD usd;

    uint256 constant DEVICE_PK = 0xD3;
    uint256 constant VERIFIER_PK = 0xBEEF;
    uint256 constant ROGUE_PK = 0xBAD;
    address device = vm.addr(DEVICE_PK);
    address verifier = vm.addr(VERIFIER_PK);
    address producer = makeAddr("producer");
    address company = makeAddr("company");

    uint32 constant HOUR = 492_000; // some past hour index
    bytes32 constant ROOT = keccak256("readings");

    function setUp() public {
        vm.warp(uint256(HOUR + 10) * 3600);
        certs = new SunmintCertificates(verifier, 860, "https://sunmint.app/api/cert/{id}.json");
        certs.setGridFactor("NG", 400);
        certs.setGridFactor("ES", 150);
        usd = new TestUSD();
        market = new SunmintMarket(certs, usd);
        _register(device, DEVICE_PK, 7200, true, "NG");
    }

    // ── helpers ──────────────────────────────────────────────────────────

    function _sign(uint256 pk, bytes32 digest) internal pure returns (bytes memory) {
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(pk, digest);
        return abi.encodePacked(r, s, v);
    }

    function _register(address d, uint256 pk, uint32 capW, bool gen, bytes2 cc) internal {
        bytes32 sh = keccak256(
            abi.encode(keccak256("Register(address owner,uint32 capacityW,bool hasGenerator,bytes2 country)"), producer, capW, gen, cc)
        );
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", _domain(), sh));
        vm.prank(producer);
        certs.registerDevice(d, capW, gen, cc, _sign(pk, digest));
    }

    function _domain() internal view returns (bytes32) {
        (, string memory name, string memory version, uint256 chainId, address vc,,) = certs.eip712Domain();
        return keccak256(
            abi.encode(
                keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
                keccak256(bytes(name)), keccak256(bytes(version)), chainId, vc
            )
        );
    }

    function _claim(uint32 hour, uint32 wh, bool gridDown) internal view returns (SunmintCertificates.HourClaim memory) {
        return SunmintCertificates.HourClaim(device, hour, wh, gridDown, ROOT);
    }

    function _sigs(SunmintCertificates.HourClaim memory c) internal view returns (bytes memory, bytes memory) {
        bytes32 d = certs.claimDigest(c);
        return (_sign(DEVICE_PK, d), _sign(VERIFIER_PK, certs.attestDigest(d)));
    }

    function _mint(SunmintCertificates.HourClaim memory c) internal returns (uint256) {
        (bytes memory a, bytes memory b) = _sigs(c);
        return certs.mint(c, a, b);
    }

    /// Signs first, so expectRevert lands on mint() and not on a digest view call.
    function _expectMintRevert(SunmintCertificates.HourClaim memory c, bytes memory err) internal {
        (bytes memory a, bytes memory b) = _sigs(c);
        vm.expectRevert(err);
        certs.mint(c, a, b);
    }

    // ── happy paths ──────────────────────────────────────────────────────

    function test_GenHourMintsAtDieselFactor() public {
        uint256 id = _mint(_claim(HOUR, 6500, true));
        (, , uint32 wh, SunmintCertificates.Source src, uint32 factor, uint64 g,) = certs.certificates(id);
        assertEq(wh, 6500);
        assertEq(uint8(src), uint8(SunmintCertificates.Source.GEN));
        assertEq(factor, 860);
        assertEq(g, 5590); // 6.5 kWh × 0.86 kg = 5.59 kg
        assertEq(certs.balanceOf(producer, id), 6500);
    }

    function test_GridHourMintsAtCountryFactor() public {
        uint256 id = _mint(_claim(HOUR, 6500, false));
        (, , , SunmintCertificates.Source src, , uint64 g,) = certs.certificates(id);
        assertEq(uint8(src), uint8(SunmintCertificates.Source.GRID));
        assertEq(g, 2600);
    }

    function test_BuyAndRetireThroughMarket() public {
        uint256 id = _mint(_claim(HOUR, 6500, true));
        vm.startPrank(producer);
        certs.setApprovalForAll(address(market), true);
        uint256 l = market.list(id, 6500, 100_000); // 0.10 tUSDC per kWh
        vm.stopPrank();

        vm.startPrank(company);
        usd.faucet(10e6);
        usd.approve(address(market), type(uint256).max);
        uint256[] memory ls = new uint256[](1);
        uint256[] memory whs = new uint256[](1);
        ls[0] = l;
        whs[0] = 3250;
        (uint256 receipt, uint256 g, uint256 cost) = market.buyAndRetire(ls, whs, "ACME Ltd");
        vm.stopPrank();

        assertEq(receipt, 1);
        assertEq(g, 2795); // half the hour
        assertEq(cost, 325_000);
        assertEq(usd.balanceOf(producer), 325_000);
        assertEq(certs.balanceOf(address(market), id), 3250);
    }

    // ── the Break it attacks ─────────────────────────────────────────────

    function test_Attack_ForgedReading() public {
        SunmintCertificates.HourClaim memory c = _claim(HOUR, 6500, false);
        bytes32 d = certs.claimDigest(c);
        bytes memory devSig = _sign(DEVICE_PK, d);
        bytes memory verSig = _sign(VERIFIER_PK, certs.attestDigest(d));
        c.wh = 7000; // edited after signing
        vm.expectRevert(SunmintCertificates.BadDeviceSignature.selector);
        certs.mint(c, devSig, verSig);
    }

    function test_Attack_RogueKey() public {
        SunmintCertificates.HourClaim memory c = _claim(HOUR, 6500, false);
        bytes32 d = certs.claimDigest(c);
        bytes memory a = _sign(ROGUE_PK, d);
        bytes memory b = _sign(VERIFIER_PK, certs.attestDigest(d));
        vm.expectRevert(SunmintCertificates.BadDeviceSignature.selector);
        certs.mint(c, a, b);
    }

    function test_Attack_NoVerifierCoSign() public {
        SunmintCertificates.HourClaim memory c = _claim(HOUR, 6500, false);
        bytes32 d = certs.claimDigest(c);
        bytes memory a = _sign(DEVICE_PK, d);
        bytes memory b = _sign(ROGUE_PK, certs.attestDigest(d));
        vm.expectRevert(SunmintCertificates.BadVerifierSignature.selector);
        certs.mint(c, a, b);
    }

    function test_Attack_AboveCapacity() public {
        _expectMintRevert(_claim(HOUR, 11_400, false), abi.encodeWithSelector(SunmintCertificates.AboveCapacity.selector, 11_400, 7560));
    }

    function test_Attack_FutureHour() public {
        uint32 now_ = uint32(block.timestamp / 3600);
        _expectMintRevert(_claim(now_, 3000, false), abi.encodeWithSelector(SunmintCertificates.HourNotOver.selector, now_));
    }

    function test_Attack_DoubleMint() public {
        _mint(_claim(HOUR, 6500, false));
        _expectMintRevert(_claim(HOUR, 6400, true), abi.encodeWithSelector(SunmintCertificates.DeviceHourAlreadyMinted.selector, device, HOUR));
    }

    function test_FakeBlackoutWithoutGeneratorGetsGridFactor() public {
        uint256 pk = 0xD4;
        address noGen = vm.addr(pk);
        _register(noGen, pk, 7200, false, "NG");
        SunmintCertificates.HourClaim memory c = SunmintCertificates.HourClaim(noGen, HOUR, 6500, true, ROOT);
        bytes32 d = certs.claimDigest(c);
        uint256 id = certs.mint(c, _sign(pk, d), _sign(VERIFIER_PK, certs.attestDigest(d)));
        (, , , SunmintCertificates.Source src, , uint64 g,) = certs.certificates(id);
        assertEq(uint8(src), uint8(SunmintCertificates.Source.GRID));
        assertEq(g, 2600);
    }

    function test_RetiredWhCannotBeRetiredTwice() public {
        uint256 id = _mint(_claim(HOUR, 6500, true));
        uint256[] memory ids = new uint256[](1);
        uint256[] memory whs = new uint256[](1);
        ids[0] = id;
        whs[0] = 6500;
        vm.startPrank(producer);
        certs.retire(ids, whs, "self");
        vm.expectRevert();
        certs.retire(ids, whs, "self");
        vm.stopPrank();
    }

    function test_RegisterNeedsDeviceKey() public {
        address d = vm.addr(0xD5);
        vm.prank(producer);
        vm.expectRevert(SunmintCertificates.BadDeviceSignature.selector);
        certs.registerDevice(d, 5000, false, "ES", _sign(ROGUE_PK, bytes32(0)));
    }
}
