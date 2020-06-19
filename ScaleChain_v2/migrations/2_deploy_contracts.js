var ScaleChain = artifacts.require("./ScaleChain.sol");

module.exports = function(deployer, network, accounts) {
	deployer.deploy(ScaleChain, accounts[0]).then(() => console.log("Contract Address: ", ScaleChain.address))

};