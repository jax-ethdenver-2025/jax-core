// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/RewardPool.sol";
import "../src/Factory.sol";
import "../src/JaxToken.sol";

contract BaseTest is Test {
    RewardPool pool;
    JaxToken jaxToken;
    Factory factory;
    address user = makeAddr("user1");
  
    function setUp() public {
        pool = new RewardPool();
        jaxToken = new JaxToken(user);
        factory = new Factory(address(pool), address(jaxToken));
    }
}
