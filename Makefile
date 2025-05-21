# http - endpoint=127.0.0.1:8545
# ws - ws://127.0.0.1:8546
geth__run:
	rm -rf data/geth;
	geth init --datadir data data/genesis.json;
	geth \
		--dev \
		--http --http.api eth,web3,net,engine --http.corsdomain "*" \
		--ws --ws.api eth,web3,net,engine \
		--datadir data \
		--password password.txt \
		--mine \
		--gcmode archive
		# --syncmode "fast" \

geth__stop:
	pkill geth

solc__build_and_deploy: solc__compile solc__depoloy

solc__compile:
	solc contract/Bridge.sol --optimize --combined-json bin,abi --via-ir > contract/combined_bridge.json
	cat contract/combined_bridge.json | jq '.contracts' | jq '."contract/Bridge.sol:DemoERC20"' > contract/DemoERC20.json
	cat contract/combined_bridge.json | jq '.contracts' | jq '."contract/Bridge.sol:Bridge"' > contract/Bridge.json
	cat contract/combined_bridge.json | jq '.contracts' | jq '."contract/console.sol:console"' > contract/console.json

solc__depoloy:
	cargo test -- test_depoloy_contracts --nocapture

solc__deposite_compile:
	solc contract/DepositeContract.sol --optimize --combined-json bin,abi --via-ir \
		| jq '.contracts' \
		| jq '."contract/DepositeContract.sol:Deposite"' \
		> contract/deposit_contract.json

contract__compile:
	cd contract; \
		npx hardhat compile

contract__test:
	cd contract; \
		npx hardhat test

contract__deploy:
	rm -r contract/ignition/deployments contract/cache contract/artifacts || true
	cd contract; \
		HARDHAT_IGNITION_CONFIRM_DEPLOYMENT=false \
		PRIVATE_KEY="cc4a7682c00703a233acab918d2e92dcdfb828663b4a8c84d4b561f6d3277ab3" \
		npx hardhat ignition deploy ./ignition/modules/Lock.js \
			--network geth \
			--show-stack-traces

fix:
	cargo fix --allow-dirty --allow-staged || exit 1
	cargo clippy --fix --no-deps --allow-dirty --allow-staged || exit 2
	cargo +nightly fmt || exit 3

check:
	cargo clippy --no-deps --all-targets -- -Dwarnings || exit 1
	cargo +nightly fmt --check || exit 2
	cargo test || exit 3

### https://gist.github.com/hosseinnedaee/48607a54acf2602ef65c97c02356b517
# geth --dev dumpgenesis --datadir data > genesis.json
# geth account new --datadir data
# geth init --datadir data data/genesis.json

### Contract
# npm install --save-dev hardhat
# npx hardhat init

### Статус развёртывания контракта
# npx hardhat ignition deployments
# npx hardhat ignition status chain-1337

### Развертывание
# solc --bin --abi -o ./build Lock.sol

