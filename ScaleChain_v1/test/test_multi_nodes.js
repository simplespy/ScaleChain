var Web3 = require('web3');
var ScaleChain = artifacts.require("./ScaleChain.sol");
var crypto = require("crypto");
const start = Date.now();
var blocks = [];
var max_length = 8;
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
    await instance.SendBlock(tx_bytes, sig);
    var updated_hash = await instance.getCurrentHash();
    //console.log(updated_hash);
    return [tx_bytes, sig, updated_hash];
    
}

async function runMainNode(account, instance) {
    while (true) {
        var block = await receiveBlocks(account, instance);
        await sleep(3000);
        blocks.push(block);
        console.log("Block[", blocks.length, "]: ", Date.now() - start, ' from ', account);
        if (blocks.length >= max_length) break;
        
    }
}

contract("ScaleChain", function(accounts) {
    it("Test 4: updating hash", async function() {
        var instance = await ScaleChain.deployed();
        var init_hash = Buffer.alloc(32);
        //await runMainNode(accounts[1], instance);
        await Promise.all([
            runMainNode(accounts[0], instance), 
            runMainNode(accounts[1], instance),
            ]);
        //validateHash(init_hash, blocks);
    });
});