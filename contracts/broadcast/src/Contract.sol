// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.13;

// TODO: incorporate some security that people
//  can't spoof the ticket
contract Contract {
    // Event for broadcasting an Iroh ticket
    event TicketBroadcast(
        string ticket,
        address sender
    );

    // Event for when a node removes a ticket
    event TicketRemoved(
        string ticket,
        address sender
    );

    // Broadcast a new Iroh ticket
    function broadcastTicket(string memory ticket) public {
        emit TicketBroadcast(ticket, msg.sender);
    }

    // Remove a previously broadcast ticket
    function removeTicket(string memory ticket) public {
        emit TicketRemoved(ticket, msg.sender);
    }
}
