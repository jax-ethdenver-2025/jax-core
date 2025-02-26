// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Initializable} from "solady/utils/Initializable.sol";
import {LibClone} from "solady/utils/LibClone.sol";
import {Ownable} from "solady/auth/Ownable.sol";
import {Pool} from "./Pool.sol";

contract Factory {
    address public immutable poolImplementation;
    uint256 public poolNonce;
    
    event PoolCreated(address indexed poolAddress, string hash);

    constructor(address _poolImplementation) {
        poolImplementation = _poolImplementation;
    }

    // TODO: add signature over hash to 
    //  ensure the hash is valid
    // TODO: advertise the node id along with the hash
    //  so that data can be retrieved from the node
    function createPool(
        string memory hash
        // assume format is raw for now
    ) external returns (address poolAddress) {
        _checksBeforeCreation();

        // TODO: add all this stuff to the pool
        poolAddress = _create();
        emit PoolCreated(poolAddress, hash);
    }

    function _checksBeforeCreation() internal view {
      // TODO Not implemented
    }

    function _create() internal returns (address poolAddress) {
        bytes32 salt = keccak256(abi.encodePacked(poolNonce));
        poolNonce++;

        poolAddress = LibClone.cloneDeterministic(poolImplementation, salt);
        Pool(poolAddress).initialize();
    }
}
