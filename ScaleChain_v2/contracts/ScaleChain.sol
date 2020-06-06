pragma solidity ^0.4.24;
pragma experimental ABIEncoderV2;

contract ScaleChain {
    mapping(address => ScaleNode) public scale_nodes;
    mapping(address => SideNode) public side_nodes;
    ScaleNode[] public scale_id;
    bytes32[] public curr_hash;
    uint[] public block_id;
    UnconfirmedBlock[] public buffer;
    uint16 public threshold;

    struct ScaleNode {
        address eth_pk;
        G2Point pub_key;
        uint32 ip_addr;
        uint index;
    }

    struct SideNode {
        address eth_pk;
        uint32 ip_addr;
        uint sid;
    }

    struct UnconfirmedBlock {
        string header;
        uint sid;
        uint bid;
    }

    struct G1Point {
        uint X;
        uint Y;
    }

    struct G2Point {
        uint[2] X;
        uint[2] Y;
    }

    function P1() internal returns (G1Point) {
        return G1Point(1, 2);
    }

    function P2() internal returns (G2Point) {
        return G2Point(
            [11559732032986387107991004021392285783925812861821192530917403151452391805634,
            10857046999023057135944570762232829481370756359578518086990519993285655852781],

            [4082367875863433681332203403145435568316851327593401208105741076214120093531,
            8495653923123431417604973247489272438418190587263600148770280649306958101930]
        );
    }

    constructor(address admin_address, uint16 t) public {
        threshold = t;
        uint index = 0;
        scale_nodes[admin_address] = ScaleNode({
            eth_pk: admin_address,
            pub_key: P2(),
            ip_addr: 0,
            index: 0
        });
        scale_id.push(scale_nodes[admin_address]);
        index += 1;
    }

    // Register a new side chain with initialized SideNode list
    function addSideChain(address[] side_node_addresses, uint32[] ip_addrs) public {
        block_id.push(0);
        curr_hash.push(0);
        uint chain_id = block_id.length;
        for (uint i = 0; i < side_node_addresses.length; ++i) {
            side_nodes[side_node_addresses[i]] = SideNode({
                eth_pk: side_node_addresses[i],
                ip_addr: ip_addrs[i],
                sid: chain_id-1
            });
        }
    }

    // Current SideNode authorize new SideNode
    // check msg.sender and not duplicate
    function addSideNode(address new_side_node, uint32 ip_addr, uint sid) public {
        require(side_nodes[msg.sender].sid == sid);
        require(side_nodes[new_side_node].sid != sid);
        side_nodes[new_side_node] = SideNode({
            eth_pk: new_side_node,
            ip_addr: ip_addr,
            sid: sid
        });
    }

    function addScaleNode(
        address new_scale_node, 
        uint pkx1, uint pkx2, 
        uint pky1, uint pky2, 
        uint32 ip_addr) public {

        require(scale_nodes[msg.sender].eth_pk != address(0), "not permitted to add");
        require(scale_nodes[new_scale_node].eth_pk == address(0), "already joined");
        uint i = scale_id.length;
        scale_nodes[new_scale_node] = ScaleNode ({
            eth_pk: new_scale_node,
            pub_key: G2Point([pkx1, pkx2], [pky1, pky2]),
            ip_addr: ip_addr,
            index: i
        });
        scale_id.push(scale_nodes[new_scale_node]);
    }

    function scaleNodesCount() public view returns (uint num_scale) {
        num_scale = scale_id.length;
    }

    function getCurrentHash(uint sid) public view
        returns (bytes32 currentHash) {
            currentHash = curr_hash[sid];
    }

    function getBlockID(uint sid) public view
        returns (uint bid) {
            bid = block_id[sid];
    }

    function splitSignature(bytes memory sig) internal pure returns (uint8 v, bytes32 r, bytes32 s) {
        require(sig.length == 65);
        assembly {
            r := mload(add(sig, 32))
            s := mload(add(sig, 64))
            v := byte(0, mload(add(sig, 96)))
        }
        if (v < 27) v = v + 27;
    }
    
    function recoverSigner(string block, bytes sig) returns (address signer_address) {
        bytes memory prefix = "\x19Ethereum Signed Message:\n32";
        bytes32 prefixedHash = sha3(prefix, sha3(block));
        (uint8 v, bytes32 r, bytes32 s) = splitSignature(sig);
        signer_address = ecrecover(prefixedHash, v, r, s);
    }
    
    function proposeBlock(string side_chain_block_header, bytes signature, uint new_block_id, uint sid) public {
        
        // 1.check signed by one of sideNode within specific side chain
        address signer_address = recoverSigner(side_chain_block_header, signature);
        require (side_nodes[signer_address].sid == sid);

        //Check whether the new_block_id = block_id + 1. If not, reject the block. 
        require (block_id[sid] + 1 == new_block_id);

        // 2. add to buffer
        buffer.push(UnconfirmedBlock({
            header: side_chain_block_header,
            sid: sid,
            bid: new_block_id
        }));
    }

    function verifyBLS(bytes message, uint sigx, uint sigy, uint pkx1, uint pkx2, uint pky1, uint pky2) returns (bool) {
        G1Point memory signature = G1Point(sigx, sigy);
        G2Point memory pub_key = G2Point([pkx1, pkx2], [pky1, pky2]);
        G1Point memory h = hashToG1(message);
        return pairing2(negate(signature), P2(), h, pub_key);
    }

    function submitVote(bytes block_header, uint sigx, uint sigy, uint bitset) public {

        // 1.check signed by one of scaleNodes
        require(scale_nodes[msg.sender].eth_pk != address(0));
        uint sid = 0;
        uint bid = 0;

        // 2.check the block header is in the buffer
        for (uint i = 0; i < buffer.length; ++i) {
            if (bytes(buffer[i].header).length == block_header.length && keccak256(buffer[i].header) == keccak256(block_header)) {
                sid = buffer[i].sid;
                bid = buffer[i].bid;
            }
        }
        require(sid > 0);

        // 3. check signature aggregation
        i = 1;
        uint bs = bitset;
        uint cnt = 0;
        while (bs > 0) {
            if (bs % 2 == 1) {
                cnt += 1;
            }
            bs /= 2;
        }
        G1Point[] memory a = new G1Point[](cnt);
        G2Point[] memory b = new G2Point[](cnt);
        a[0] = negate(G1Point(sigx, sigy));
        b[0] = P2();
        G1Point memory h0 = hashToG1(block_header);
        bs = bitset;
        while (bs > 0) {
            if (bs % 2 == 1) {
                a[i] = h0;
                b[i] = scale_id[i].pub_key;
            }
            i += 1;
            bs /= 2;
        }
        require(pairing(a, b));

        // If pass, update hash
        bytes32 new_hash = sha256(curr_hash[sid], sha256(block_header));
        curr_hash[sid] = new_hash;
        block_id[sid] = block_id[sid] + 1;


    }



      function pairing(G1Point[] p1, G2Point[] p2) internal returns (bool) {
        require(p1.length == p2.length);
        uint elements = p1.length;
        uint inputSize = elements * 6;
        uint[] memory input = new uint[](inputSize);

        for (uint i = 0; i < elements; i++)
        {
            input[i * 6 + 0] = p1[i].X;
            input[i * 6 + 1] = p1[i].Y;
            input[i * 6 + 2] = p2[i].X[0];
            input[i * 6 + 3] = p2[i].X[1];
            input[i * 6 + 4] = p2[i].Y[0];
            input[i * 6 + 5] = p2[i].Y[1];
        }

        uint[1] memory out;
        bool success;

        assembly {
            success := call(sub(gas, 2000), 8, 0, add(input, 0x20), mul(inputSize, 0x20), out, 0x20)
        // Use "invalid" to make gas estimation work
            switch success case 0 {invalid}
        }
        require(success);
        return out[0] != 0;
    }

    function pairing2(G1Point a1, G2Point a2, G1Point b1, G2Point b2) internal returns (bool) {
        G1Point[] memory p1 = new G1Point[](2);
        G2Point[] memory p2 = new G2Point[](2);
        p1[0] = a1;
        p1[1] = b1;
        p2[0] = a2;
        p2[1] = b2;
        return pairing(p1, p2);
    }

    function hashToG1(bytes message) internal returns (G1Point) {
        uint256 h = uint256(keccak256(message));
        return mul(P1(), h);
    }

    function modPow(uint256 base, uint256 exponent, uint256 modulus) internal returns (uint256) {
        uint256[6] memory input = [32, 32, 32, base, exponent, modulus];
        uint256[1] memory result;
        assembly {
            if iszero(call(not(0), 0x05, 0, input, 0xc0, result, 0x20)) {
                revert(0, 0)
            }
        }
        return result[0];
    }

    /// @return the negation of p, i.e. p.add(p.negate()) should be zero.
    function negate(G1Point p) internal returns (G1Point) {
        // The prime q in the base field F_q for G1
        uint q = 21888242871839275222246405745257275088696311157297823662689037894645226208583;
        if (p.X == 0 && p.Y == 0)
            return G1Point(0, 0);
        return G1Point(p.X, q - (p.Y % q));
    }

    /// @return the sum of two points of G1
    function add(G1Point p1, G1Point p2) internal returns (G1Point r) {
        uint[4] memory input;
        input[0] = p1.X;
        input[1] = p1.Y;
        input[2] = p2.X;
        input[3] = p2.Y;
        bool success;
        assembly {
            success := call(sub(gas, 2000), 6, 0, input, 0xc0, r, 0x60)
        // Use "invalid" to make gas estimation work
            switch success case 0 {invalid}
        }
        require(success);
    }
    /// @return the product of a point on G1 and a scalar, i.e.
    /// p == p.mul(1) and p.add(p) == p.mul(2) for all points p.
    function mul(G1Point p, uint s) internal returns (G1Point r) {
        uint[3] memory input;
        input[0] = p.X;
        input[1] = p.Y;
        input[2] = s;
        bool success;
        assembly {
            success := call(sub(gas, 2000), 7, 0, input, 0x80, r, 0x60)
        // Use "invalid" to make gas estimation work
            switch success case 0 {invalid}
        }
        require(success);
    }

}
