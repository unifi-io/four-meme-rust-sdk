import { ethers } from "ethers";
import { whatsabi } from "@shazow/whatsabi";

const apiKey = '3QRJYBGC1K6D6PKRWTNYACC7R179FYKGDG';

const provider = new ethers.JsonRpcProvider("https://bsc.blockrazor.xyz");


async function main() {
    const proxy = "0x5c952063c7fc8610ffdb798152d69f0b9550762b"; // Example: Four.meme TokenManager proxy
    // EIP-1967 implementation slot: bytes32(uint256(keccak256('eip1967.proxy.implementation')) - 1)
    const implSlot = "0x360894A13BA1A3210667C828492DB98DCA3E2076CC3735A920A3CA505D382BBC";
    const raw = await provider.getStorage(proxy, implSlot);
    const impl = ethers.getAddress("0x" + raw.slice(26));  // Current implementation address
    console.log('impl: ', impl);
    const result = await whatsabi.autoload(impl, { provider });
    console.log('result: ', JSON.stringify(result.abi, null, 2));
}

main();

// Use BscScan API to fetch implementation contract ABI
// const url = `https://api.bscscan.com/v2/api?module=contract&action=getabi&address=${impl}&apikey=${process.env.BSCSCAN_KEY}`;
/*
const url = `https://api.etherscan.io/v2/api?chainid=56&module=contract&action=getabi&address=${impl}&apikey=${apiKey}`;
console.log('url: ', url);

const res = await fetch(url).then(r => r.json());
const abi = JSON.parse(res.result);

console.log('abi: ', abi);
*/