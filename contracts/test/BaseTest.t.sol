// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {console2} from "forge-std/Test.sol";

import "forge-std/Test.sol";
import "../src/RewardPool.sol";
import "../src/Factory.sol";
import "../src/JaxToken.sol";
import "../test/mocks/AVSMock.sol";

contract BaseTest is Test {
    RewardPool pool;
    JaxToken jaxToken;
    Factory factory;
    AVSMock avs;
    address user = makeAddr("user1");
  
    function setUp() public {
        address poolImplementation = address(new RewardPool());
        jaxToken = new JaxToken(user);
        avs = new AVSMock();
        factory = new Factory(poolImplementation, address(jaxToken), address(avs));

        pool = RewardPool(factory.createPool("test", "user1"));
    }

    function test_verifySignature() public {
        bytes32 k = 0x06cf14cfae0ff9fe7fdf773202029a3e8976465c8919f4840d1c3c77c8162435;
        bytes32 r = 0xa6161c95fd4e3237b7dd12cc3052aaa69382510ecb5b89c2fbeb8b6efb78266b;
        bytes32 s = 0x81160af2842235a0257fc1d3e968c2c1c9f56f117da3186effcaeda256c38a0d;
        bytes memory m = hex"b0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc40943e2c10f2ad4ee49ab0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc";
        factory.verifySignature(k, r, s, m);
    }

    function test_verifyAgainstAVS() public {
        bytes32 data = keccak256(abi.encodePacked(user));
        vm.expectCall(address(avs), abi.encodeCall(IAVS.verify, (data)));
        pool.verifyAgainstAVS(data);
    }

    function test_getWalletProviders() public {
        vm.expectCall(address(avs), abi.encodeCall(IAVS.getWalletProviders, ()));
        pool.getWalletProviders();
    }
}
