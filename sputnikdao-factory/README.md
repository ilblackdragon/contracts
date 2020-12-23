# SputnikDAO Factory

# Deployment & Usage

## TestNet

```
near dev-deploy --wasmFile=res/sputnikdao_factory.wasm

# bash
ARGS=`echo '{"purpose": "test", "council": ["testmewell.testnet", "illia"], "bond": "1000000000000000000000000", "vote_period": "1800000000000", "grace_period": "1800000000000"}' | base64`
# fish
set ARGS (echo '{"purpose": "test", "council": ["testmewell.testnet", "illia"], "bond": "1000000000000000000000000", "vote_period": "1800000000000", "grace_period": "1800000000000"}' | base64)

# Create new DAO with given parameters.
near call dev-1608694678554-8567049 create "{\"name\": \"test\", \"args\": \"$ARGS\"}"  --accountId dev-1608694678554-8567049 --amount 30

# List all created DAOs.
near view dev-1608694678554-8567049 get_dao_list
```

