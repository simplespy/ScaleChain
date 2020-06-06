var Web3 = require('web3');
var ScaleChain = artifacts.require("./ScaleChain.sol");
var crypto = require("crypto");


function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function signBlock(tx_bytes, signer) {
    var hash = web3.utils.soliditySha3(tx_bytes);
    return web3.eth.sign(hash, signer).then(token => { return token });      
}

function validateHash(init_hash, blocks) {
    var curr_hash = init_hash;
    blocks.forEach(function (item, index, array) {
        var [tx_bytes, sig, hash] = item
        var tx_hash = crypto.createHash('sha256').update(tx_bytes).digest();
        var buffer = Buffer.concat([curr_hash, tx_hash]);
        var new_hash = crypto.createHash('sha256').update(buffer);
        assert.equal(new_hash.digest('hex'), hash.slice(2));
        curr_hash = crypto.createHash('sha256').update(buffer).digest();
    });
}

async function receiveBlocks(node_account, instance) {
    var tx_bytes = ""
    while (tx_bytes.length < 32) {
        var len = Math.floor(Math.random() * 10)
        var new_bytes = crypto.randomBytes(len).toString('hex');
        tx_bytes += new_bytes;
    }
    var sig = await signBlock(tx_bytes, node_account);
    await instance.proposeBlock(tx_bytes, sig);
    var updated_hash = await instance.getCurrentHash();
    console.log(updated_hash);
    return [tx_bytes, sig, updated_hash];
    
}

contract("ScaleChain", function(accounts) {
    it ("Test 1: initializes with admin account", function() {
        return ScaleChain.deployed().then(function(instance) {
            return instance.scaleNodesCount();

        }).then(function(count) {
            console.log(count);
            assert.equal(count.toNumber(), 1);
        });
    });

    it("Test 2: add account", function() {
        ScaleChain.deployed().then(function(instance) {
            web3.eth.defaultAccount = accounts[0];
            instance.addScaleNode(accounts[1], 1, 2, 3, 4, 5);
            return instance.scaleNodesCount();
        }).then(function(count) {
            assert.equal(count.toNumber(), 2);
        });
    });

    it("Test 3: signing blocks", async function() {
        var instance = await ScaleChain.deployed();
        var tx_bytes = "deadbeef";
        var sig = await signBlock(tx_bytes, accounts[0]);
        var result = await instance.recoverSigner.call(tx_bytes, sig);
        assert.equal(result, accounts[0]);
    });

    it("Test 4: updating hash", async function() {
        var instance = await ScaleChain.deployed();
        var init_hash = Buffer.alloc(32);
        var blocks = [];
        await instance.addSideChain([accounts[2], accounts[3]], [6001, 6002]);
        var hash = await instance.getCurrentHash.call(0);
        var bid = await instance.getBlockID.call(0);
        console.log(hash, bid);
       /* while (true) {
            var block = await receiveBlocks(accounts[0], instance);
            await sleep(2000);
            blocks.push(block);
            if (blocks.length >= 10) break;
        }
        validateHash(init_hash, blocks);*/
    });
});