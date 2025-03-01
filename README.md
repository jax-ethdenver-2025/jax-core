# jax-core

[jax](https://jax.ac/): the permissionlessly incentivized file storage layer for Ethereum

jax provides a decentralized storage network where anyone can participate as a storage provider and earn rewards. Built on Ethereum, it ensures your files remain accessible, censorship-resistant, and secure.

this repo contains the core logic for our node implementation which:
- is responsible for discovering peers
- opting into storage tasks that are available on the network
- participating in off-chain eigentrust updates via blake3 hash gossip and probing
- storing and serving files
- interacting with the Ethereum blockchain to claim rewards

this node is meant to run next to an eigenlayer AVS for orchestrating submissions of eigentrust updates on
chain and collecting rewards.

by running this node, you are opting into the storage tasks on the network and agreeing to store files for others.
peers will be able to find you once you have announced a share to the network or otherwise announced 
your collaboration with a pool.

peers will continually probe you to ensure you are still online and participating in the network, forming 
an eigentrust consensus value for your node in order to determine if you are a reliable peer and to 
determine your relative storage reward.

## running

to set up a new node for the first time, run:

```bash
cargo run --bin jax -- init \
    # the eigen layer avs contract address
    --factory-address $FACTORY_ADDRESS \
    # the private key for the node -- if not provided, will use a random key
    --eth-signer $NODE_KEY
```

this will spin up a new iroh peer and on disk config in the xdg config directory (~/.config/jax/jax.conf)

see [the avs repo](https://github.com/jax-ethdenver-2025/jax-avs-go) for more information on how to run an avs
and defining the factory address.

to run the node, run:

```bash
cargo run --bin jax -- node
```

## interacting with the node

we provide two main interfaces for interacting with the node:

### cli

to interact with the node, you can use the following commands:

```bash
alias jax="cargo run --bin jax -- "
jax --help
```

you can check the status of the node with:

```bash
$ jax status
Server Status:
- Node ID: 3f2fc3ec32b6cde1e454858598549a812b8020816cdfdf6472429d935eb50a62
- ETH Address: 0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC
```

and start making content available on the network with:

```bash
$ jax share -p examples/fade.jpg
File '/Users/al/krondor/jax/jax-core/examples/fade.jpg' has been added to the blob store and announced to the network
Share ticket: blobaa7s7q7mgk3m3ypekscylgcutkasxabaqfwn7x3eojbj3e26wufgeaaaaauf24wb532z7djjj33oefveiorrahqbwnlzmb3y3frowpueest2o
Hash: fboxfqpo6wpy2kko63rbnjcdumib4antk6lao6gzmlvt5bbeu6tq
```

optionally you can also create a pool when you create a share:

```bash
$ jax share -p examples/fade.jpg -c
```

which will initialize a reward pool for the share and automatically opt you into the storage task.

nodes will automatically join pools that they are interested in (by default they will join all pools)

### web interface (!)

by default the node will start a web server on port 8080. you can access the web interface at:

```bash
http://localhost:8080/
```

this will show a simple web interface for interacting with the node, allowing you to share files,
join pools, and view the status of the network.

## development

to run the node in development mode, run:

```bash
cargo run --bin jax -- dev
```

## testing

to run the tests, run:

```bash
cargo test
```

## formatting

to format the code, run:

```bash
cargo fmt
```

## linting

to run the linting, run:

```bash
cargo clippy
```

## contributing

we welcome contributions! please see the [contributing guide](CONTRIBUTING.md) for more information.

## license

this project is licensed under the [MIT license](LICENSE).
