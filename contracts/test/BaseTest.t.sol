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

        bytes32 h = 0xfe817b5da668b5b0576b6f1fda0e8a3cbe2546e551447c780f8f606b994ad514;

        pool = RewardPool(factory.createPool{value: 1 ether}(h));
    }

    function test_poolExists() public view {
        uint256 poolCodeSize = address(pool).code.length;
        assertGt(poolCodeSize, 0);
    }

    function test_poolBalance() public view {
        assertEq(address(pool).balance, 1 ether);
    }

    function test_verifySignature() public view {
        bytes32 k = 0x3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29;
        bytes32 r = 0x7aff0db8d8ae6261b6726c5d8216182554f5ba4b249687da0289c5d1afc8f6aa;
        bytes32 s = 0xef6f50c132af4317cb2631da1f4704a64ebdbc902528d160c25e289bbd8c650c;
        bytes memory m =
            hex"f39fd6e51aad88f6f4ce6ab8827279cfffb92266";
        bool verification = pool.verify(k, r, s, m);
        assertEq(verification, true);
    }

    function test_getTaskResponse() public {
        vm.expectCall(address(avs), abi.encodeCall(IIncredibleSquaringTaskManager.getTaskResponse, (1)));
        pool.getTaskResponse(1);
    }

    function test_deposit() public {
        uint256 depositAmount = 1 ether;
        uint256 targetAmount = 2 ether;
        vm.deal(user, depositAmount);

        vm.prank(user);
        pool.deposit{value: depositAmount}();

        assertEq(pool.getBalance(), targetAmount);
    }

    function test_validPoolEntrance() public {
        string memory nodeId = "node1";
        bytes32 k = 0x3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29;
        bytes32 r = 0x7aff0db8d8ae6261b6726c5d8216182554f5ba4b249687da0289c5d1afc8f6aa;
        bytes32 s = 0xef6f50c132af4317cb2631da1f4704a64ebdbc902528d160c25e289bbd8c650c;
        address m = 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266;
        pool.enterPool(nodeId, k, r, s, m);

        string[] memory peers = pool.getPeers();
        assertEq(peers.length, 1);
        assertEq(peers[0], nodeId);
    }
}
