# ScaleChain

### Steps

**[./ScaleChain_v1]**

1. deploy the contract on Ropsten
	* command: `truffle deploy --network ropsten [--reset]`
	* address: [0xe5c10a1e39fa1faf25e4fd5ce2c4e2ec5a7ab926](https://ropsten.etherscan.io/address/0xe5c10a1e39fa1faf25e4fd5ce2c4e2ec5a7ab926)
	* update contract address in `mainNode-config_1.js`

**[./mainNode]**

1. `npm install` 
2. Print contract status info: `node mainNode_1.js -i`

3. Test addMainNode
	* command: `node mainNode_1.js -a 0x97952767d3748FA35443c6319eA0ef192C5d1a76` 
	* transaction: [0xf777c1f27cd48871ba9b7b15965c3e8b591b1e36f3308fd1b85b3082ddabf61c](https://ropsten.etherscan.io/tx/0xf777c1f27cd48871ba9b7b15965c3e8b591b1e36f3308fd1b85b3082ddabf61c)
4. Test sendBlock
	* command: `node mainNode_1.js -s`
	* check hash: `node mainNode_1.js -h`	
	* Transaction: [0xd09dbae11f3112b908d42bdbf063e27b4b07a2197bb1c95dd729e9cdc0ca671f](https://ropsten.etherscan.io/tx/0xd09dbae11f3112b908d42bdbf063e27b4b07a2197bb1c95dd729e9cdc0ca671f)
5. Retrieve transaction history and recompute hash
	* `node mainNode_1.js -v`
	
---
	
### Test Commands		
```
Options:
  --version             Show version number                            [boolean]
  --addMainNode, -a     Add a main node to member list
  --getMainNode, -g     Get main node given index (from 1)              [number]
  --countMainNodes, -c  Get the number of main nodes
  --getCurrentHash, -h  Get the current hash value
  --getBlockID, -b      Get the current block ID
  --sendBlock, -s       Send a random block
  --test, -t            Testing code
  --info, -i            Get current info
  --verify, -v          Retrieve transaction history to verify block hash
  --help                Show help                                      [boolean]
```