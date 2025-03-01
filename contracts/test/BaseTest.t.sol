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
        bytes32 k = 0x06cf14cfae0ff9fe7fdf773202029a3e8976465c8919f4840d1c3c77c8162435;
        bytes32 r = 0xa6161c95fd4e3237b7dd12cc3052aaa69382510ecb5b89c2fbeb8b6efb78266b;
        bytes32 s = 0x81160af2842235a0257fc1d3e968c2c1c9f56f117da3186effcaeda256c38a0d;
        bytes memory m =
            hex"b0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc40943e2c10f2ad4ee49ab0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc";
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
        bytes32 r = 0x63145222a1f366ce657e65989b4f7074a4bc4093d2ba71ff1a3b72fcc40de4f4;
        bytes32 s = 0x3707e1e40de32db295cb584f6a6f060f19c35dd69046686a560315b5d25e9106;
        bytes memory m =
            hex"63145222a1f366ce657e65989b4f7074a4bc4093d2ba71ff1a3b72fcc40de4f43707e1e40de32db295cb584f6a6f060f19c35dd69046686a560315b5d25e9106";
        pool.enterPool(nodeId, k, r, s, m);

        string[] memory peers = pool.getPeers();
        assertEq(peers.length, 1);
        assertEq(peers[0], nodeId);
    }
}
