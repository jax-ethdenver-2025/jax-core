pragma solidity ^0.8.20;

interface IAVS {
    function getWalletProviders() external view returns (address[] memory);

    function verify(bytes32 data) external view returns (bool);
}