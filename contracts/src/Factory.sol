// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Ed25519} from "./libraries/ED25519.sol";
import {Initializable} from "solady/utils/Initializable.sol";
import {LibClone} from "solady/utils/LibClone.sol";
import {Ownable} from "solady/auth/Ownable.sol";
import {RewardPool} from "./RewardPool.sol";

contract Factory {
    address public immutable poolImplementation;
    address public immutable avs;
    // Add storage for pools
    address[] public pools;
    mapping(address => bool) public isPool;
    // Add mapping for hash tracking
    mapping(bytes32 => bool) public usedHashes;

    /* Events */

    event PoolCreated(address indexed poolAddress, bytes32 hash, uint256 balance);

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

    function createPool(
        bytes32 hash
    ) external payable returns (address poolAddress) {
        // Check if hash is already used
        require(!usedHashes[hash], "Hash already used");
        
        poolAddress = _create(hash, msg.value);
        
        // Track the new pool and hash
        pools.push(poolAddress);
        isPool[poolAddress] = true;
        usedHashes[hash] = true;
        
        emit PoolCreated(poolAddress, hash, msg.value);
    }

    function _create(
        bytes32 hash,
        uint256 value
    ) internal returns (address poolAddress) {
        bytes32 salt = hash;

        poolAddress = LibClone.cloneDeterministic(poolImplementation, salt);
        RewardPool(poolAddress).initialize{value: value}(avs, hash);
    }
}
