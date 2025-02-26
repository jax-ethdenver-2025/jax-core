// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Factory} from "../src/Factory.sol";
import {Pool} from "../src/Pool.sol";

contract FactoryScript is Script {
    Factory public factory;
    Pool public pool;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        address poolImplementation = address(new Pool());
        factory = new Factory(poolImplementation);

        address poolAddress = factory.createPool();
        pool = Pool(poolAddress);

        vm.stopBroadcast();
    }
}
