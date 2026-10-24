// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ERC1155Holder} from "@openzeppelin/contracts/token/ERC1155/utils/ERC1155Holder.sol";
import {SunmintCertificates} from "./SunmintCertificates.sol";

/// @title Fixed-price market for SUNMINT certificates
/// @notice Sellers escrow Wh here; buyers pay in a stablecoin and can retire in the same call.
contract SunmintMarket is ERC1155Holder {
    using SafeERC20 for IERC20;

    struct Listing {
        address seller;
        uint256 id;
        uint256 wh;          // remaining
        uint256 pricePerKwh; // stablecoin base units
    }

    SunmintCertificates public immutable certs;
    IERC20 public immutable usd;
    Listing[] public listings;

    event Listed(uint256 indexed listing, address indexed seller, uint256 indexed id, uint256 wh, uint256 pricePerKwh);
    event Bought(uint256 indexed listing, address indexed buyer, uint256 wh, uint256 cost);
    event Cancelled(uint256 indexed listing);
    event RetiredFor(uint256 indexed receipt, address indexed buyer, uint256 cost);

    error NotSeller();
    error NotEnough(uint256 available);

    constructor(SunmintCertificates certs_, IERC20 usd_) {
        certs = certs_;
        usd = usd_;
    }

    function listingCount() external view returns (uint256) {
        return listings.length;
    }

    function list(uint256 id, uint256 wh, uint256 pricePerKwh) external returns (uint256 listing) {
        certs.safeTransferFrom(msg.sender, address(this), id, wh, "");
        listing = listings.length;
        listings.push(Listing(msg.sender, id, wh, pricePerKwh));
        emit Listed(listing, msg.sender, id, wh, pricePerKwh);
    }

    function cancel(uint256 listing) external {
        Listing storage l = listings[listing];
        if (l.seller != msg.sender) revert NotSeller();
        uint256 wh = l.wh;
        l.wh = 0;
        certs.safeTransferFrom(address(this), msg.sender, l.id, wh, "");
        emit Cancelled(listing);
    }

    function quote(uint256 listing, uint256 wh) public view returns (uint256) {
        return (wh * listings[listing].pricePerKwh + 999) / 1000;
    }

    /// @notice Buy Wh from several listings; certificates go to the buyer.
    function buy(uint256[] calldata ls, uint256[] calldata whs) external returns (uint256 cost) {
        for (uint256 i; i < ls.length; ++i) cost += _take(ls[i], whs[i], msg.sender);
    }

    /// @notice Buy and retire in one transaction, so the buyer never holds the Wh.
    function buyAndRetire(uint256[] calldata ls, uint256[] calldata whs, string calldata beneficiary)
        external
        returns (uint256 receipt, uint256 gCO2, uint256 cost)
    {
        uint256[] memory ids = new uint256[](ls.length);
        for (uint256 i; i < ls.length; ++i) {
            cost += _take(ls[i], whs[i], address(this));
            ids[i] = listings[ls[i]].id;
        }
        (receipt, gCO2) = certs.retire(ids, whs, beneficiary);
        emit RetiredFor(receipt, msg.sender, cost);
    }

    function _take(uint256 listing, uint256 wh, address to) private returns (uint256 cost) {
        Listing storage l = listings[listing];
        if (wh > l.wh) revert NotEnough(l.wh);
        l.wh -= wh;
        cost = quote(listing, wh);
        usd.safeTransferFrom(msg.sender, l.seller, cost);
        if (to != address(this)) certs.safeTransferFrom(address(this), to, l.id, wh, "");
        emit Bought(listing, msg.sender, wh, cost);
    }
}
