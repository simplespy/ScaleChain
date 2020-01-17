var Web3 = require('web3');
var ScaleChain = artifacts.require("./ScaleChain.sol");



function signBlock(contents, nonce, contractAddress, callback) {
    var hash = "0x" + web3.utils.soliditySha3(
        {t: "bytes32", v: contents}, 
        {t: "uint256", v:nonce}, 
        {t: 'address', v: contractAddress}
    ).toString("hex");

    web3.eth.sign(hash, web3.eth.defaultAccount, callback);
      
}

// hash = soliditySha3(transactions + contractAddress)
// sig = sign(hash, signer's key)
// block = {transactions, sig}
// block_ser = ser(block)


contract("ScaleChain", function(accounts) {
    it ("Test 1: initializes with accounts", function() {
        return ScaleChain.deployed().then(function(instance) {
            return instance.etherNodesCount();
        }).then(function(count) {
            assert.equal(count.toNumber(), 5);
        });
    });

    it("Test 2: add account", function() {
        ScaleChain.deployed().then(function(instance) {
            web3.eth.defaultAccount = accounts[0];
            instance.addEtherNode(accounts[5]);
            return instance.etherNodesCount();
        }).then(function(count) {
            assert.equal(count.toNumber(), 6);
        });
    });

    it("Test 3: signing blocks", function() {
        return ScaleChain.deployed().then(function(instance) {
            web3.eth.defaultAccount = accounts[0];
            const block = "";//new ArrayBuffer(32);
            signBlock(block, 100, instance.address, function(err, sig) {
                assert.equal(instance.SendBlock(block, sig), web3.eth.defaultAccount);
            });
        });
    });
});