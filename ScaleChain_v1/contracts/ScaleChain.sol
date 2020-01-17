pragma solidity ^0.4.24;
pragma experimental ABIEncoderV2;

contract ScaleChain {
    EtherNode[] public ether_Nodes;
    mapping(address => bool) is_etherNode;
    bytes32 curr_hash;

    struct EtherNode {
        address id;
        bytes32 pubkey;
    }

    struct SendResult {
        bool result; //success or not
        bytes32 tx_hash; //maybe useful
    }

    // Initialize EtherNodes so that a sender of a new block
    // has to be one of the etherNode
    constructor(address[] ether_node_addresses) public {
        for (uint i = 0; i < ether_node_addresses.length; ++i) {
            ether_Nodes.push(EtherNode({
                id: ether_node_addresses[i],
                pubkey: 0
            }));
            is_etherNode[ether_node_addresses[i]] = true;
        }
    }

    // Current EtherNode authorize new EtherNode
    // Note: check msg.sender and not duplicate
    function addEtherNode(address new_ether_node) public {
        require(is_etherNode[msg.sender]);
        require(!is_etherNode[new_ether_node]);
        ether_Nodes.push(EtherNode({
            id: new_ether_node,
            pubkey: 0
        }));
        is_etherNode[new_ether_node] = true;
    }

    // Get Ether node addresses
    function getEtherNodes(uint id) public view
        returns (address ether_node_addresses) {
            ether_node_addresses = ether_Nodes[id].id;
    }

    function getCurrentHash() public view
        returns (bytes32 currentHash) {
            currentHash = curr_hash;
    }

    // Get # of Ether nodes
    function etherNodesCount() public view
        returns (uint number_of_etherNodes) {
            number_of_etherNodes = ether_Nodes.length;
    }

    function splitSignature(bytes memory sig) internal pure returns (uint8 v, bytes32 r, bytes32 s) {
        require(sig.length == 65);
        assembly {
            r := mload(add(sig, 32))
            s := mload(add(sig, 64))
            v := byte(0, mload(add(sig, 96)))
        }
    }

    function recoverSigner(bytes32 contents, bytes memory signature) internal pure returns (address signer_address) {
        
        (uint8 v, bytes32 r, bytes32 s) = splitSignature(signature);
        signer_address = ecrecover(contents, v, r, s);
    }

    function prefixed(bytes32 hash) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked("\x19Ethereum Signed Message:\n32", hash));
    }
    
    function toBytes(bytes32 _data) internal pure returns (bytes memory b) {
        b = new bytes(_data.length);
        for(uint i = 0; i < _data.length; ++i) {
            b[i] = _data[i];
        }
        return b;
    }
    
    // compute hash
    function SendBlock(bytes memory tx_bytes, bytes memory sig) public returns (SendResult) {
        
        // 1.check signed by one of etherNode
        bytes32 contents = prefixed(keccak256(tx_bytes));
        address signer_address = recoverSigner(contents, sig);

        require (is_etherNode[signer_address]);
        
        // 2. update hash tentative codes
        bytes32 h1 = sha256(abi.encodePacked(tx_bytes));
        bytes32 h2 = sha256(abi.encodePacked(curr_hash, h1));
        curr_hash = h2;
        
        // 3. publish the curr_hash as part of transaction   
        bool publish_result = address(this).call();
        return SendResult({result: publish_result, tx_hash: curr_hash});
    }

    function TestSignerAddress(bytes memory tx_bytes, bytes memory sig) public returns (address signer_address) {
        bytes32 contents = prefixed(keccak256(abi.encodePacked(tx_bytes)));
        signer_address = recoverSigner(contents, sig);
    }

}