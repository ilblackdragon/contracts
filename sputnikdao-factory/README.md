# SputnikDAO Factory

# Deployment & Usage

## TestNet

```
near dev-deploy --wasmFile=res/sputnikdao_factory.wasm

# bash
CONTRACT_ID="dev-1608694678554-8567049"
# fish
set CONTRACT_ID "dev-1608694678554-8567049"

# Initialize the factory.
near call $CONTRACT_ID new '{}' --accountId $CONTRACT_ID 

# bash
ARGS=`echo '{"purpose": "test", "council": ["testmewell.testnet", "illia"], "bond": "1000000000000000000000000", "vote_period": "1800000000000", "grace_period": "1800000000000"}' | base64`
# fish
set ARGS (echo '{"purpose": "test", "council": ["testmewell.testnet", "illia"], "bond": "1000000000000000000000000", "vote_period": "1800000000000", "grace_period": "1800000000000"}' | base64)

# Create new DAO with the given parameters.
near call $CONTRACT_ID create "{\"name\": \"test\", \"args\": \"$ARGS\"}"  --accountId $CONTRACT_ID --amount 30

# List all created DAOs.
near view $CONTRACT_ID get_dao_list
```

