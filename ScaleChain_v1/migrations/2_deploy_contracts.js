var ScaleChain = artifacts.require("./ScaleChain.sol");

module.exports = function(deployer, network, accounts) {
	deployer.deploy(ScaleChain, accounts);
};