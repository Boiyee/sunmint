// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {Script, console} from "forge-std/Script.sol";
import {SunmintCertificates} from "../src/SunmintCertificates.sol";
import {SunmintMarket} from "../src/SunmintMarket.sol";
import {TestUSD} from "../src/TestUSD.sol";

/// forge script script/Deploy.s.sol --rpc-url base_sepolia --broadcast --verify
/// env: PRIVATE_KEY (deployer), VERIFIER_ADDRESS, METADATA_URI
contract Deploy is Script {
    function run() external {
        vm.startBroadcast(vm.envUint("PRIVATE_KEY"));

        // Demo emission factors, g CO2 per kWh. Replace with sourced values before judging.
        SunmintCertificates certs =
            new SunmintCertificates(vm.envAddress("VERIFIER_ADDRESS"), 860, vm.envString("METADATA_URI"));
        certs.setGridFactor("NG", 400);
        certs.setGridFactor("IN", 710);
        certs.setGridFactor("PH", 650);
        certs.setGridFactor("ES", 150);
        certs.setGridFactor("KE", 100);
        certs.setGridFactor("BR", 90);

        TestUSD usd = new TestUSD();
        SunmintMarket market = new SunmintMarket(certs, usd);

        vm.stopBroadcast();

        console.log("SunmintCertificates", address(certs));
        console.log("TestUSD           ", address(usd));
        console.log("SunmintMarket     ", address(market));
    }
}
