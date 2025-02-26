// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.28;

import {ERC20} from "solady/tokens/ERC20.sol";
import {Initializable} from "solady/utils/Initializable.sol";
import {Ownable} from "solady/auth/Ownable.sol";

/**
 * @title JAX Token Contract
 * @notice Tracks and manages JAX token for users in the system.
 */
contract JaxToken is ERC20, Initializable, Ownable {
    constructor(address initialOwner) {
        _mint(initialOwner, type(uint256).max);

        _disableInitializers();
        _initializeOwner(initialOwner);
    }

    function name() public pure override returns (string memory) {
        return "JAXToken";
    }

    function symbol() public pure override returns (string memory) {
        return "JAX";
    }

    function decimals() public pure override returns (uint8) {
        return 18;
    }
}
