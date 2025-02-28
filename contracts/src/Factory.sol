// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Ed25519} from "./libraries/ED25519.sol";
import {Initializable} from "solady/utils/Initializable.sol";
import {LibClone} from "solady/utils/LibClone.sol";
import {Ownable} from "solady/auth/Ownable.sol";
import {RewardPool} from "./RewardPool.sol";

contract Factory {
    address public immutable poolImplementation;
    uint256 public poolNonce;
    address public immutable avs;
    // Add storage for pools
    address[] public pools;
    mapping(address => bool) public isPool;

    /* Events */

    event PoolCreated(address indexed poolAddress, string hash);

    /* Constructor */

    constructor(address _poolImplementation, address _avs) {
        poolImplementation = _poolImplementation;
        avs = _avs;
    }

    // Add function to get all pools
    function getAllPools() external view returns (address[] memory) {
        return pools;
    }

    /* Public Functions */

    // TODO: add signature verification
    function createPool(
        string memory hash
    ) external returns (address poolAddress) {
        poolAddress = _create(hash);
        
        // Track the new pool
        pools.push(poolAddress);
        isPool[poolAddress] = true;
        
        emit PoolCreated(poolAddress, hash);
    }

    function _create(
        string memory hash
    ) internal returns (address poolAddress) {
        bytes32 salt = keccak256(abi.encodePacked(poolNonce));
        poolNonce++;

        poolAddress = LibClone.cloneDeterministic(poolImplementation, salt);
        RewardPool(poolAddress).initialize(avs, hash);
    }
}
