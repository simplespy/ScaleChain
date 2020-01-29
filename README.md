# ScaleChain

### Steps

[./ScaleChain_v1]

1. deploy the contract on Ropsten
	* command: `ScaleChain_v1 spy$ truffle deploy --network ropsten [--reset]`
	* address: [0x200faF75Ce05656Fe7DCCcBD647CcC2b3A7b2eca](https://ropsten.etherscan.io/address/0x200faF75Ce05656Fe7DCCcBD647CcC2b3A7b2eca)
	* update contract address in `mainNode_1.js`

[./mainNode]

1. `npm install` 

2. Test addMainNode
	* command: `mainNode_1 spy$ node mainNode_1.js -a 0x97952767d3748FA35443c6319eA0ef192C5d1a76` 
	* transaction: [0x1784d0f087dd8389818a29bddd65dde47cdc3cec32481b7282cbd69a42939698](https://ropsten.etherscan.io/tx/0x1784d0f087dd8389818a29bddd65dde47cdc3cec32481b7282cbd69a42939698)
3. Test sendBlock
	* command: `mainNode_1 spy$ node mainNode_1.js -s`
	* check hash: `mainNode_1 spy$ node mainNode_1.js -h`	
	* Transaction: [0x5d5012bc14be76a1f6fce4f8e4b51f99918c3fa657e99e39cabcb301a3ef8f10](https://ropsten.etherscan.io/tx/0x5d5012bc14be76a1f6fce4f8e4b51f99918c3fa657e99e39cabcb301a3ef8f10)
	
---
	
### Test Commands		
```
node mainNode_1.js --help
Options:
  --version             Show version number                            [boolean]
  --addMainNode, -a     Add a main node to member list
  --getMainNode, -g     Get main node given index                       [number]
  --countMainNodes, -c  Get the number of main nodes
  --getCurrentHash, -h  Get the current hash value
  --getBlockID, -b      Get the current block ID
  --sendBlock, -s       Send a random block
  --help                Show help                                      [boolean]
```