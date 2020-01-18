var Web3 = require('web3');
var ScaleChain = artifacts.require("./ScaleChain.sol");



function signBlock(tx_bytes, signer) {
    var hash = web3.utils.soliditySha3(tx_bytes);
    return web3.eth.sign(hash, signer).then(token => { return token });      
}

contract("ScaleChain", function(accounts) {
    it ("Test 1: initializes with accounts", function() {
        return ScaleChain.deployed().then(function(instance) {
            return instance.mainNodesCount();
        }).then(function(count) {
            assert.equal(count.toNumber(), 5);
        });
    });

    it("Test 2: add account", function() {
        ScaleChain.deployed().then(function(instance) {
            web3.eth.defaultAccount = accounts[0];
            instance.addMainNode(accounts[5]);
            return instance.mainNodesCount();
        }).then(function(count) {
            assert.equal(count.toNumber(), 6);
        });
    });

    it("Test 3: signing blocks", async function() {
        var instance = await ScaleChain.deployed();
        var tx_bytes = "deadbeef";
        var sig = await signBlock(tx_bytes, accounts[0]);
        var result = await instance.recoverSigner.call(tx_bytes, sig);
        assert.equal(result, accounts[0]);
    });
});