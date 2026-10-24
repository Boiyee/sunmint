// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {ERC1155} from "@openzeppelin/contracts/token/ERC1155/ERC1155.sol";
import {EIP712} from "@openzeppelin/contracts/utils/cryptography/EIP712.sol";
import {ECDSA} from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/// @title SUNMINT hourly clean-energy certificates
/// @notice One token id per device-hour. Balances are in Wh, so certificates can be split and partly retired.
///         Minting needs the meter's own EIP-712 signature AND the verifier's co-signature; the displaced
///         source (grid vs generator) and the CO2 weight are decided here, on-chain, from public rules.
contract SunmintCertificates is ERC1155, EIP712, Ownable {
    enum Source { GRID, GEN }

    struct Device {
        address owner;
        uint32 capacityW;     // array nameplate, watts
        bool hasGenerator;    // registered backup generator at the site
        bytes2 country;       // ISO 3166-1 alpha-2, picks the grid factor
    }

    struct HourClaim {
        address device;
        uint32 hour;          // unix time / 3600
        uint32 wh;            // energy produced in that hour
        bool gridDown;        // signed grid channel read 0 V for the whole hour
        bytes32 readingsRoot; // Merkle root of the hour's 5-minute signed readings
    }

    struct Certificate {
        address device;
        uint32 hour;
        uint32 wh;
        Source source;
        uint32 factor;        // g CO2 per kWh applied
        uint64 gCO2;          // CO2 avoided, grams
        bytes32 readingsRoot;
    }

    bytes32 private constant REGISTER_TYPEHASH =
        keccak256("Register(address owner,uint32 capacityW,bool hasGenerator,bytes2 country)");
    bytes32 private constant CLAIM_TYPEHASH =
        keccak256("HourClaim(address device,uint32 hour,uint32 wh,bool gridDown,bytes32 readingsRoot)");
    bytes32 private constant ATTEST_TYPEHASH = keccak256("Attest(bytes32 claimDigest)");

    /// 5% tolerance over nameplate for one hour of output.
    uint256 private constant CAPACITY_TOLERANCE_BPS = 10_500;

    address public verifier;
    uint32 public genFactor;                      // g/kWh for a small diesel genset
    mapping(bytes2 => uint32) public gridFactor;  // g/kWh per country
    mapping(address => Device) public devices;
    mapping(uint256 => Certificate) public certificates;
    uint256 public receiptCount;

    event DeviceRegistered(address indexed device, address indexed owner, uint32 capacityW, bool hasGenerator, bytes2 country);
    event Minted(uint256 indexed id, address indexed device, uint32 hour, uint32 wh, Source source, uint64 gCO2, bytes32 readingsRoot);
    event Retired(uint256 indexed receipt, address indexed by, string beneficiary, uint256[] ids, uint256[] whs, uint256 gCO2);
    event VerifierSet(address verifier);
    event FactorSet(bytes2 indexed country, uint32 gPerKwh);

    error UnknownDevice(address device);
    error DeviceExists(address device);
    error BadDeviceSignature();
    error BadVerifierSignature();
    error HourNotOver(uint32 hour);
    error AboveCapacity(uint32 wh, uint256 maxWh);
    error NoGridFactor(bytes2 country);
    error DeviceHourAlreadyMinted(address device, uint32 hour);

    constructor(address verifier_, uint32 genFactor_, string memory uri_)
        ERC1155(uri_)
        EIP712("SUNMINT", "1")
        Ownable(msg.sender)
    {
        verifier = verifier_;
        genFactor = genFactor_;
        emit VerifierSet(verifier_);
    }

    // ── admin ────────────────────────────────────────────────────────────

    function setVerifier(address v) external onlyOwner {
        verifier = v;
        emit VerifierSet(v);
    }

    function setGridFactor(bytes2 country, uint32 gPerKwh) external onlyOwner {
        gridFactor[country] = gPerKwh;
        emit FactorSet(country, gPerKwh);
    }

    function setGenFactor(uint32 gPerKwh) external onlyOwner {
        genFactor = gPerKwh;
        emit FactorSet("GN", gPerKwh);
    }

    // ── devices ──────────────────────────────────────────────────────────

    /// @notice The meter proves it holds its key by signing who owns it and what it is.
    function registerDevice(address device, uint32 capacityW, bool hasGenerator, bytes2 country, bytes calldata deviceSig)
        external
    {
        if (devices[device].owner != address(0)) revert DeviceExists(device);
        if (gridFactor[country] == 0) revert NoGridFactor(country);
        bytes32 digest = _hashTypedDataV4(
            keccak256(abi.encode(REGISTER_TYPEHASH, msg.sender, capacityW, hasGenerator, country))
        );
        if (ECDSA.recover(digest, deviceSig) != device) revert BadDeviceSignature();
        devices[device] = Device(msg.sender, capacityW, hasGenerator, country);
        emit DeviceRegistered(device, msg.sender, capacityW, hasGenerator, country);
    }

    // ── minting ──────────────────────────────────────────────────────────

    function claimDigest(HourClaim calldata c) public view returns (bytes32) {
        return _hashTypedDataV4(
            keccak256(abi.encode(CLAIM_TYPEHASH, c.device, c.hour, c.wh, c.gridDown, c.readingsRoot))
        );
    }

    function attestDigest(bytes32 claimDigest_) public view returns (bytes32) {
        return _hashTypedDataV4(keccak256(abi.encode(ATTEST_TYPEHASH, claimDigest_)));
    }

    function certificateId(address device, uint32 hour) public pure returns (uint256) {
        return uint256(keccak256(abi.encode(device, hour)));
    }

    /// @notice Anyone may submit; the signatures carry the authority. Mints to the device owner.
    function mint(HourClaim calldata c, bytes calldata deviceSig, bytes calldata verifierSig) external returns (uint256 id) {
        Device memory d = devices[c.device];
        if (d.owner == address(0)) revert UnknownDevice(c.device);

        bytes32 digest = claimDigest(c);
        if (ECDSA.recover(digest, deviceSig) != c.device) revert BadDeviceSignature();
        if (ECDSA.recover(attestDigest(digest), verifierSig) != verifier) revert BadVerifierSignature();

        if ((uint256(c.hour) + 1) * 3600 > block.timestamp) revert HourNotOver(c.hour);
        uint256 maxWh = uint256(d.capacityW) * CAPACITY_TOLERANCE_BPS / 10_000;
        if (c.wh > maxWh) revert AboveCapacity(c.wh, maxWh);

        id = certificateId(c.device, c.hour);
        if (certificates[id].device != address(0)) revert DeviceHourAlreadyMinted(c.device, c.hour);

        // Displacement rule: generator only with a signed blackout AND a registered generator.
        Source source = c.gridDown && d.hasGenerator ? Source.GEN : Source.GRID;
        uint32 factor = source == Source.GEN ? genFactor : gridFactor[d.country];
        uint64 gCO2 = uint64(uint256(c.wh) * factor / 1000);

        certificates[id] = Certificate(c.device, c.hour, c.wh, source, factor, gCO2, c.readingsRoot);
        emit Minted(id, c.device, c.hour, c.wh, source, gCO2, c.readingsRoot);
        _mint(d.owner, id, c.wh, "");
    }

    // ── retirement ───────────────────────────────────────────────────────

    /// @notice Burns the caller's Wh and emits a public receipt. Retired Wh can never be claimed again.
    function retire(uint256[] calldata ids, uint256[] calldata whs, string calldata beneficiary)
        external
        returns (uint256 receipt, uint256 gCO2)
    {
        _burnBatch(msg.sender, ids, whs);
        for (uint256 i; i < ids.length; ++i) {
            if (whs[i] == 0) continue;
            Certificate storage cert = certificates[ids[i]];
            gCO2 += uint256(cert.gCO2) * whs[i] / cert.wh;
        }
        receipt = ++receiptCount;
        emit Retired(receipt, msg.sender, beneficiary, ids, whs, gCO2);
    }
}
