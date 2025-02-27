// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {JaxToken} from "../src/JaxToken.sol";

contract JaxTokenScript is Script {
    JaxToken public jaxToken;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        jaxToken = new JaxToken(msg.sender);

        vm.stopBroadcast();
    }
}
