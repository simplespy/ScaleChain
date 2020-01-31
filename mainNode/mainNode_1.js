const Web3 = require('web3');
const fs = require('fs');
const yargs = require('yargs');
var crypto = require("crypto");
var Tx = require('ethereumjs-tx').Transaction;
var assert = require('assert');
var api = require('etherscan-api').init('UGEFW13C4HVZ9GGH5GWIRHQHYYPYKX7FCX','ropsten');
const abiDecoder = require('abi-decoder'); 

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
	.option('test', {
		alias: 't',
		description: 'Testing code'
	})
	.option('info', {
		alias: 'i',
		description: 'Get current info'
	})
	.option('verify', {
		alias: 'v',
		description: 'Retrieve transaction history to verify block hash'
	})
	.help()
	.argv;

const rpcURL = 'https://ropsten.infura.io/v3/fd7fd90847cd4ca99ce886d4bffdccf8';
const web3 = new Web3(rpcURL);
const config = JSON.parse(fs.readFileSync('mainNode-config_1.json', 'utf8'));
const contract_address = config.contract_address;

const address = config.mainNode_1.address;
const key = config.mainNode_1.private_key;
const key_to_sign = Buffer.from(key.slice(2), 'hex');

const contract = JSON.parse(fs.readFileSync(config.contract_abi_path, 'utf8'));
const instance = new web3.eth.Contract(contract.abi, contract_address);
abiDecoder.addABI(contract.abi);

var stored_hash = new Buffer(32);

async function get_all(func_sig) {
	var curr_hash = Buffer.alloc(32);
	var result = Buffer.alloc(32);
	let txs = await api.account.txlist(contract_address, 1, 'latest', 1, 100, 'asc');
	for (tx of txs.result) {
		if (tx.input.slice(2, 10) == func_sig) {
			let func_abi = abiDecoder.decodeMethod(tx.input);
			let block = func_abi.params[0].value;
			var block_hash = crypto.createHash('sha256').update(block).digest();
        	var buffer = Buffer.concat([curr_hash, block_hash]);
        	curr_hash = crypto.createHash('sha256').update(buffer).digest();
        	result = crypto.createHash('sha256').update(buffer);
		}
	}
	return result.digest('hex');
}

function receiveBlock(block_size) {
    var tx_bytes = ""
    while (tx_bytes.length < block_size) {
        var len = Math.floor(Math.random() * 10);
        var new_bytes = crypto.randomBytes(len).toString('hex');
        tx_bytes += new_bytes;
    }
    return tx_bytes;
}

if (argv.a) {
	address_to_add = config.mainNode_2.address;
	const functionABI = instance.methods.addMainNode(address_to_add).encodeABI();
	web3.eth.getTransactionCount(address).then(txCount => {
		const txData = {
			nonce: web3.utils.toHex(txCount),
		    to: contract_address,
		    from: address,
		    value: web3.utils.toHex(web3.utils.toWei('0.0', 'ether')),
		    data: functionABI,
		    gasLimit: web3.utils.toHex(100000),
		    gasPrice: web3.utils.toHex(web3.utils.toWei('10', 'gwei'))
	 	}
	 	transaction = new Tx(txData, { chain: 'ropsten' })
	    transaction.sign(key_to_sign);
	    const serializedTx = transaction.serialize().toString('hex');
	    web3.eth.sendSignedTransaction('0x' + serializedTx, (err, txHash) => {
	    	if (err) console.log(err)
	    	else console.log('txHash:', txHash)
	  	})
    })
}

if (argv.g) instance.methods.getMainNode(argv.g-1).call((err, result) => { console.log(result) })
if (argv.c) instance.methods.mainNodesCount().call((err, result) => { console.log(result) })
if (argv.h) instance.methods.getCurrentHash().call((err, result) => { console.log(result) })
if (argv.b) instance.methods.getBlockID().call((err, result) => { console.log(result) })
if (argv.i) {
	instance.methods.mainNodesCount().call((err, result) => { console.log("Number of mainNodes = ", result) })
	instance.methods.getCurrentHash().call((err, result) => { console.log("Current Hash = ", result) })
	instance.methods.getBlockID().call((err, result) => { console.log("Current BlockID = ", result) })
}


if (argv.s) {
	instance.methods.getBlockID().call((err, blk_id) => {
		const block = receiveBlock(config.side_chain_block_size);
		console.log("block = ", block)
		const hash = web3.utils.soliditySha3({t: "string", v: block});
		let sig = web3.eth.accounts.sign(hash, key);
		const new_blk_id = parseInt(blk_id) + 1;
		const functionABI = instance.methods.sendBlock(block, sig.signature, new_blk_id).encodeABI();
		web3.eth.getTransactionCount(address).then(txCount => {
			const txData = {
				nonce: web3.utils.toHex(txCount),
			    to: contract_address,
			    value: web3.utils.toHex(web3.utils.toWei('0.0', 'ether')),
			    data: functionABI,
			    gasLimit: web3.utils.toHex(100000),
			    gasPrice: web3.utils.toHex(web3.utils.toWei('10', 'gwei'))
			}
	 		transaction = new Tx(txData, { chain: 'ropsten' })
	    	transaction.sign(key_to_sign);
	    	const serializedTx = transaction.serialize().toString('hex');
		    web3.eth.sendSignedTransaction('0x' + serializedTx)
		    	.on('transactionHash', console.log)
		    	.on('error', console.log);
    	})
	})
}

if (argv.v) {
	const function_sig = 'ae8d0145'
	get_all(function_sig).then(computed_hash => {
		console.log("recomputed hash = ", "0x" + computed_hash);
		instance.methods.getCurrentHash().call((err, result) => { console.log("hash from contract = ", result) })
	});
	
}

