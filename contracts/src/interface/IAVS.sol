pragma solidity ^0.8.20;

interface IAVS {
    function getWalletProviders() external view returns (address[] memory);
}