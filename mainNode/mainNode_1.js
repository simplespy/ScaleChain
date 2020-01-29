const Web3 = require('web3');
const fs = require('fs');
const yargs = require('yargs');
var crypto = require("crypto");
var Tx = require('ethereumjs-tx').Transaction;

const argv = yargs
	.option('addMainNode',  {
		alias: 'a',
		description: 'Add a main node to member list',
		type: 'String'
	})
	.option('getMainNode', {
		alias: 'g',
		description: 'Get main node given index (from 1)',
		type: 'number'
	})
	.option('countMainNodes', {
		alias: 'c',
		description: 'Get the number of main nodes',
	})
	.option('getCurrentHash', {
		alias: 'h',
		description: 'Get the current hash value',
	})
	.option('getBlockID', {
		alias: 'b',
		description: 'Get the current block ID'
	})
	.option('sendBlock', {
		alias: 's',
		description: 'Send a random block'
	})
	.help()
	.argv;

const rpcURL = 'https://ropsten.infura.io/v3/fd7fd90847cd4ca99ce886d4bffdccf8';
const web3 = new Web3(rpcURL);
const contract_address = '0x200faF75Ce05656Fe7DCCcBD647CcC2b3A7b2eca';

// mainNode_1 is not member
const address1 = '0x97952767d3748FA35443c6319eA0ef192C5d1a76';
const privateKey1 = Buffer.from('05BBB87FD8ED92EC8938B96A46459B21B06FDCEAF6FC4C0C42A2C5A36AECA77E', 'hex')

//const privateKey1 = Buffer.from(process.env.PRIVATE_KEY_1, 'hex');
// mainNode_2 is member
const address2 = '0x30DBcCEb2096A3a8A1d39FD626e7a4aA2D98895a';
const privateKey2 = Buffer.from('E02F6780A5F946DA0D818984E8EE4058BBDB3400DB79396615C5B7B314FF6DFE', 'hex');

const contract = JSON.parse(fs.readFileSync('../ScaleChain_v1/build/contracts/ScaleChain.json', 'utf8'));
const instance = new web3.eth.Contract(contract.abi, contract_address);

if (argv.a) {
	address_to_add = argv.a
	const functionABI = instance.methods.addMainNode(address_to_add).encodeABI();
	web3.eth.getTransactionCount(address2).then(txCount => {
		const txData = {
			nonce: web3.utils.toHex(txCount),
		    to: contract_address,
		    value: web3.utils.toHex(web3.utils.toWei('0.0', 'ether')),
		    data: functionABI,
		    gasLimit: web3.utils.toHex(300000),
		    gasPrice: web3.utils.toHex(web3.utils.toWei('10', 'gwei'))
	 	}
	 	transaction = new Tx(txData, { chain: 'ropsten' })
	    transaction.sign(privateKey1);
	    const serializedTx = transaction.serialize().toString('hex');
	    web3.eth.sendSignedTransaction('0x' + serializedTx, (err, txHash) => {
	    	if (err) console.log(err)
	    	else console.log('txHash:', txHash)
	  	})
    })
}

if (argv.g) instance.methods.getMainNodes(argv.g-1).call((err, result) => { console.log(result) })
if (argv.c) instance.methods.mainNodesCount().call((err, result) => { console.log(result) })
if (argv.h) instance.methods.getCurrentHash().call((err, result) => { console.log(result) })
if (argv.b) instance.methods.getBlockID().call((err, result) => { console.log(result) })

function receiveBlock(block_size) {
    var tx_bytes = ""
    while (tx_bytes.length < block_size) {
        var len = Math.floor(Math.random() * 10)
        var new_bytes = crypto.randomBytes(len).toString('hex');
        tx_bytes += new_bytes;
    }
    return tx_bytes;
}

if (argv.s) {
	instance.methods.getBlockID().call((err, blk_id) => {
		const block = receiveBlock(32);
		console.log("block = ", block)
		const hash = web3.utils.soliditySha3(block);
		const signature = web3.eth.accounts.sign(hash, privateKey2)
		msg_hash = web3.utils.toHex(signature['messageHash'])
		v = signature['v']
		r = signature['r']
		s = signature['s']
		const functionABI = instance.methods.SendBlock(block, msg_hash, v, r, s, blk_id + 1).encodeABI();
		web3.eth.getTransactionCount(address2).then(txCount => {
			const txData = {
				nonce: web3.utils.toHex(txCount),
			    to: contract_address,
			    value: web3.utils.toHex(web3.utils.toWei('0.0', 'ether')),
			    data: functionABI,
			    gasLimit: web3.utils.toHex(300000),
			    gasPrice: web3.utils.toHex(web3.utils.toWei('10', 'gwei'))
			}
	 		transaction = new Tx(txData, { chain: 'ropsten' })
	    	transaction.sign(privateKey2);
	    	const serializedTx = transaction.serialize().toString('hex');
		    web3.eth.sendSignedTransaction('0x' + serializedTx, (err, txHash) => {
		    	if (err) console.log(err)
		    	else console.log('txHash:', txHash)
		  	})
    	})
	})
}



