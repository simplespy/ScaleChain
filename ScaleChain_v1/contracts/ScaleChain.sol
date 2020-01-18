pragma solidity ^0.4.24;
pragma experimental ABIEncoderV2;

contract ScaleChain {
    MainNode[] public main_nodes;
    mapping(address => bool) is_mainNode;
    bytes32 curr_hash;

    struct MainNode {
        address id;
        bytes32 pubkey;
    }

    struct SendResult {
        bool result; //success or not
        bytes32 tx_hash; //maybe useful
    }

    // Initialize MainNodes so that a sender of a new block
    // has to be one of the mainNode
    constructor(address[] main_node_addresses) public {
        for (uint i = 0; i < main_node_addresses.length; ++i) {
            main_nodes.push(MainNode({
                id: main_node_addresses[i],
                pubkey: 0
            }));
            is_mainNode[main_node_addresses[i]] = true;
        }
    }

    // Current MainNode authorize new MainNode
    // Note: check msg.sender and not duplicate
    function addMainNode(address new_main_node) public {
        require(is_mainNode[msg.sender]);
        require(!is_mainNode[new_main_node]);
        main_nodes.push(MainNode({
            id: new_main_node,
            pubkey: 0
        }));
        is_mainNode[new_main_node] = true;
    }

    // Get Ether node addresses
    function getMainNodes(uint id) public view
        returns (address main_node_address) {
            main_node_address = main_nodes[id].id;
    }

    function getCurrentHash() public view
        returns (bytes32 currentHash) {
            currentHash = curr_hash;
    }

    // Get # of Ether nodes
    function mainNodesCount() public view
        returns (uint number_of_mainNodes) {
            number_of_mainNodes = main_nodes.length;
    }

    function splitSignature(bytes memory sig) internal pure returns (uint8 v, bytes32 r, bytes32 s) {
        require(sig.length == 65);
        assembly {
            r := mload(add(sig, 32))
            s := mload(add(sig, 64))
            v := byte(0, mload(add(sig, 96)))
        }
        v = v + 27;
    }
    
    // compute hash
    function SendBlock(string tx_bytes, bytes memory sig) public returns (SendResult) {
        
        // 1.check signed by one of mainNode
        address signer_address = recoverSigner(tx_bytes, sig);

        require (is_mainNode[signer_address]);
        
        // 2. update hash tentative codes
        bytes32 new_hash = sha256(abi.encodePacked(curr_hash, sha256(tx_bytes)));
        curr_hash = new_hash;
        
        // 3. publish the curr_hash as part of transaction   
        bool publish_result = address(this).call();
        return SendResult({result: publish_result, tx_hash: curr_hash});
    }

    function recoverSigner(string tx_bytes, bytes memory sig) returns (address signer_address) {
        bytes memory prefix = "\x19Ethereum Signed Message:\n32";
        bytes32 prefixedHash = sha3(prefix, sha3(tx_bytes));
        (uint8 v, bytes32 r, bytes32 s) = splitSignature(sig);
        signer_address = ecrecover(prefixedHash, v, r, s);
    }


    

}