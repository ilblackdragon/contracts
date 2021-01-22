# Generic Factory

**Currently requires way too much gas for initialization. Tracking https://github.com/near/NEPs/issues/137**

Factory contract that is initialized with with byte code of the contract to be created.

Methods:
 - `new(code: BaseU8Vec)` - initializes the factory with the code of the contract to create.
 - `get_owner() -> AccountId` - returns current owner
 - `set_owner(owner_id: AccountId)` - only owner, can set new owner
 - `create(name: AccountId, args: BaseU8Vec)` - creates new contract and calls `new` with given args.
 - `update(code: BaseU8Vec)` - only owner, update code inside the factory.

# Deployment

## TestNet

### Deploy factory 

```javascript
const accountId = "illia";
const contractName = "factory2.illia";
const fs = require('fs');
const account = await near.account(accountId);
const code = fs.readFileSync("../sputnikdao/res/sputnikdao.wasm");
// const newArgs = {"owner": accountId, "code": code.toString('base64')};
// const args = Buffer.from(JSON.stringify(newArgs));
let lenBuffer = new Buffer.allocUnsafe(4);
lenBuffer.writeUInt32LE(code.length);
const args = Buffer.concat([
    Buffer.from([accountId.length, 0, 0, 0]),
    Buffer.from(accountId),
    lenBuffer,
    code,
]);
account.signAndSendTransaction(
    contractName,
    [
        nearAPI.transactions.createAccount(),
        nearAPI.transactions.transfer("100000000000000000000000000"),  
        nearAPI.transactions.deployContract(fs.readFileSync("res/generic_factory.wasm")),
        nearAPI.transactions.functionCall("new", args, 210000000000000, "0"),
    ]);
```

### Upgrade factory contract

```javascript
const accountId = "illia";
const contractName = "factory2.illia";
const fs = require('fs');
const account = await near.account(accountId);
const code = fs.readFileSync("../sputnikdao/res/sputnikdao.wasm");
let lenBuffer = new Buffer.allocUnsafe(4);
lenBuffer.writeUInt32LE(code.length);
const args = Buffer.concat([
    lenBuffer,
    code,
]);
account.signAndSendTransaction(
    contractName,
    [
        nearAPI.transactions.functionCall("upgrade", args, 210000000000000, "0"),
    ]);
```
