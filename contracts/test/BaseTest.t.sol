// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import {console2} from "forge-std/Test.sol";

import "forge-std/Test.sol";
import "../src/RewardPool.sol";
import "../src/Factory.sol";
import "../test/mocks/AVSMock.sol";

contract BaseTest is Test {
    RewardPool pool;
    Factory factory;
    AVSMock avs;
    address user = makeAddr("user1");
  
    function setUp() public {
        address poolImplementation = address(new RewardPool());
        avs = new AVSMock();
        factory = new Factory(poolImplementation, address(avs));

        bytes memory h = hex"b0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc40943e2c10f2ad4ee49ab0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc";

        pool = RewardPool(factory.createPool{value: 1 ether}(string(h)));
    }

    function test_poolExists() public view {
        uint256 poolCodeSize = address(pool).code.length;
        assertGt(poolCodeSize, 0);
    }

    function test_poolBalance() public view {
        assertEq(address(pool).balance, 1 ether);
    }

    function test_verifySignature() public view {
        bytes32 k = 0x06cf14cfae0ff9fe7fdf773202029a3e8976465c8919f4840d1c3c77c8162435;
        bytes32 r = 0xa6161c95fd4e3237b7dd12cc3052aaa69382510ecb5b89c2fbeb8b6efb78266b;
        bytes32 s = 0x81160af2842235a0257fc1d3e968c2c1c9f56f117da3186effcaeda256c38a0d;
        bytes memory m = hex"b0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc40943e2c10f2ad4ee49ab0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc";
        pool.verify(k, r, s, m);
    }

    function test_getTaskResponse() public {
        vm.expectCall(address(avs), abi.encodeCall(IIncredibleSquaringTaskManager.getTaskResponse, (1)));
        pool.getTaskResponse(1);
    }

    function test_deposit() public {
        uint256 depositAmount = 1 ether;
        vm.deal(user, depositAmount);
        
        vm.prank(user);
        pool.deposit{value: depositAmount}();
        
        assertEq(pool.balances(user), depositAmount);
    }

    function test_setBountyPerEpoch() public {
        uint256 bounty = 1 ether;
        vm.prank(pool.owner());
        pool.setBountyPerEpoch(bounty);
        
        assertEq(pool.bountyPerEpoch(), bounty);
    }

    function test_revertIfNotOWnerSetBounty() public {
        vm.prank(makeAddr("notOwner"));
        vm.expectRevert();
        pool.setBountyPerEpoch(1 ether);
    }

    function test_validPoolEntrance() public {
        string memory nodeId = "node1";
        bytes32 k = 0x06cf14cfae0ff9fe7fdf773202029a3e8976465c8919f4840d1c3c77c8162435;
        bytes32 r = 0xa6161c95fd4e3237b7dd12cc3052aaa69382510ecb5b89c2fbeb8b6efb78266b;
        bytes32 s = 0x81160af2842235a0257fc1d3e968c2c1c9f56f117da3186effcaeda256c38a0d;
        bytes memory m = hex"b0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc40943e2c10f2ad4ee49ab0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc";
        Signature memory signature = Signature(k, r, s, m);
        pool.enterPool(nodeId, signature);

        string[] memory peers = pool.getAllPeers();
        assertEq(peers.length, 1);
        assertEq(peers[0], nodeId);
    }
}
