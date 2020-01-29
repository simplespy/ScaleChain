var ScaleChain = artifacts.require("./ScaleChain.sol");

module.exports = function(deployer, network, accounts) {
	deployer.deploy(ScaleChain, accounts.slice(0,5)).then(() => console.log("Contract Address: ", ScaleChain.address))

};