// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.20;

import "../src/libraries/ED25519.sol";

contract TestEd25519 {

    function test_verify_regular_pub_regular_msg_regular_sig() public pure returns (bool) {
        bytes32 k = 0x06cf14cfae0ff9fe7fdf773202029a3e8976465c8919f4840d1c3c77c8162435;
        bytes32 r = 0xa6161c95fd4e3237b7dd12cc3052aaa69382510ecb5b89c2fbeb8b6efb78266b;
        bytes32 s = 0x81160af2842235a0257fc1d3e968c2c1c9f56f117da3186effcaeda256c38a0d;
        bytes memory m = hex"b0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc40943e2c10f2ad4ee49ab0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc";
        return Ed25519.verify(k, r, s, m);
    }

    function test_verify_regular_pub_regular_msg_invalid_sig() public pure returns (bool) {
        bytes32 k = 0x06cf14cfae0ff9fe7fdf773202029a3e8976465c8919f4840d1c3c77c8162435;
        bytes32 r = 0xb6161c95fd4e3237b7dd12cc3052aaa69382510ecb5b89c2fbeb8b6efb78266b;
        bytes32 s = 0x81160af2842235a0257fc1d3e968c2c1c9f56f117da3186effcaeda256c38a0d;
        bytes memory m = hex"b0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc40943e2c10f2ad4ee49ab0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc";
        return Ed25519.verify(k, r, s, m);
    }

    function test_verify_regular_pub_invalid_msg_regular_sig() public pure returns (bool) {
        bytes32 k = 0x06cf14cfae0ff9fe7fdf773202029a3e8976465c8919f4840d1c3c77c8162435;
        bytes32 r = 0xa6161c95fd4e3237b7dd12cc3052aaa69382510ecb5b89c2fbeb8b6efb78266b;
        bytes32 s = 0x81160af2842235a0257fc1d3e968c2c1c9f56f117da3186effcaeda256c38a0d;
        bytes memory m = hex"a0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc40943e2c10f2ad4ee49ab0d8bdfd9f4d1023dae836b2e41da5019d20c60965dc";
        return Ed25519.verify(k, r, s, m);
    }
}