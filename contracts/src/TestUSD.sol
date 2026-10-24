// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @notice Testnet-only stablecoin with an open faucet. Never deploy to mainnet.
contract TestUSD is ERC20 {
    constructor() ERC20("Test USD Coin", "tUSDC") {}

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    function faucet(uint256 amount) external {
        require(amount <= 10_000e6, "max 10k per call");
        _mint(msg.sender, amount);
    }
}
