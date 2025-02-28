// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Factory} from "../src/Factory.sol";
import {RewardPool} from "../src/RewardPool.sol";

contract FactoryScript is Script {
    Factory public factory;
    RewardPool public pool;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        address poolImplementation = address(new RewardPool());
        address avs = address(0);
        factory = new Factory(poolImplementation, avs);

        vm.stopBroadcast();
    }
}
