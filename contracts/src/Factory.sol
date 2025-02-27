// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Initializable} from "solady/utils/Initializable.sol";
import {LibClone} from "solady/utils/LibClone.sol";
import {Ownable} from "solady/auth/Ownable.sol";
import {RewardPool} from "./RewardPool.sol";

contract Factory {
    address public immutable poolImplementation;
    uint256 public poolNonce;
    address public immutable jaxToken;

    /* Events */

    event PoolCreated(address indexed poolAddress, string hash, string nodeId);

    /* Constructor */

    constructor(address _poolImplementation, address _jaxToken) {
        poolImplementation = _poolImplementation;
        jaxToken = _jaxToken;
    }

    /* Public Functions */

    // TODO: add signature verification
    function createPool(
        string memory hash,
        string memory nodeId
    ) external returns (address poolAddress) {
        _checksBeforeCreation();

        poolAddress = _create();
        emit PoolCreated(poolAddress, hash, nodeId);
    }

    function _checksBeforeCreation() internal view {
      // TODO Not implemented
    }

    function _create() internal returns (address poolAddress) {
        bytes32 salt = keccak256(abi.encodePacked(poolNonce));
        poolNonce++;

        poolAddress = LibClone.cloneDeterministic(poolImplementation, salt);
        RewardPool(poolAddress).initialize(jaxToken);
    }
}
