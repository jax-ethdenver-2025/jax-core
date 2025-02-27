// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Factory} from "../src/Factory.sol";
import {JaxToken} from "../src/JaxToken.sol";
import {RewardPool} from "../src/RewardPool.sol";

contract FactoryScript is Script {
    Factory public factory;
    RewardPool public pool;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        address poolImplementation = address(new RewardPool());
        address jaxToken = address(new JaxToken(msg.sender));
        factory = new Factory(poolImplementation, jaxToken);

        vm.stopBroadcast();
    }
}
