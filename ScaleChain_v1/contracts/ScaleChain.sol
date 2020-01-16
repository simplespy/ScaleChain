pragma solidity ^0.4.24;
pragma experimental ABIEncoderV2;

contract ScaleChain {
    EtherNode[] public ether_Nodes;
    mapping(address => bool) is_etherNode;
    bytes32 curr_hash;
    
    struct SideChainBlock {
        bytes[] transactions;
        bytes32 signature;
    }

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
    function getEtherNodes(uint id, uint balance) public view
        returns (address ether_node_addresses) {
            ether_node_addresses = ether_Nodes[id].id;
    }

    // Get # of Ether nodes
    function etherNodesCount() public view
        returns (uint number_of_etherNodes) {
            number_of_etherNodes = ether_Nodes.length;
    }
    
    // Get block
    function deserialize(bytes32 block_ser) returns (SideChainBlock) {
        return SideChainBlock({
            transactions: new bytes[](32),
            signature: block_ser
        });
    }
    
    // compute hash
    function SendBlock(bytes32 block_ser) public returns (SendResult) {
        // 0. deserialize to SideChainBlock
        //block = deserialize(block_ser);
        // 1.check signed by one of etherNode
        // use recoverSigner(), more info https://solidity.readthedocs.io/en/v0.4.24/solidity-by-example.html#safe-remote-purchase
       
        // 2. update hash tentative codes
        // bytes32 h1 = sha256(block_ser);
        // bytes32 h2 = sha256(prev_block_hash);
        // curr_hash = h2;
        
        // 3. publish the curr_hash as part of transaction   
        // use address.call() more info https://solidity.readthedocs.io/en/latest/units-and-global-variables.html#mathematical-and-cryptographic-functions
        return SendResult({result: true, tx_hash: 0});
    } 

}