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

```javascript
const accountId = "illia";
const contractName = "factory1.illia";
const fs = require('fs');
const account = await near.account(accountId);
const newArgs = {"owner": accountId, "code": fs.readFileSync("../sputnikdao/res/sputnikdao.wasm").toString('base64')};
account.signAndSendTransaction(
    contractName,
    [
        nearAPI.transactions.createAccount(),
        nearAPI.transactions.transfer("100000000000000000000000000"),  
        nearAPI.transactions.deployContract(fs.readFileSync("res/generic_factory.wasm")),
        nearAPI.transactions.functionCall("new", Buffer.from(JSON.stringify(newArgs)), 210000000000000, "0"),
    ]);
```
