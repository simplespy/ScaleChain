var Web3 = require('web3');
var ScaleChain = artifacts.require("./ScaleChain.sol");


var hash = web3.utils.sha3("message to sign");

function signBlock(signAddress, contents, nonce, contractAddress, callback) {
    var hash = "0x" + web3.utils.soliditySha3(
        {t: "uint256", v: contents}, 
        {t: "uint256", v:nonce}, 
        {t: 'address', v: contractAddress}
    ).toString("hex");
    web3.eth.sign(hash, signAddress, callback);

}

contract("ScaleChain", function(accounts) {
    it("Test 1: initializes with accounts", function() {
        return ScaleChain.deployed().then(function(instance) {
            return instance.etherNodesCount();
        }).then(function(count) {
            assert.equal(count.toNumber(), 10);
        });
    });

    it("Test 2: signing blocks", function() {
        return ScaleChain.deployed().then(function(instance) {
            signBlock(accounts[0], 100, 100, instance.address, function(err, signature) {
                console.log(signature);
            });
            return instance.etherNodesCount();
        }).then(function(count) {
            assert.equal(count.toNumber(), 10);
        });
    });
});