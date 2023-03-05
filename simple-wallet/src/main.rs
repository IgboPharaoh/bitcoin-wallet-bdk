mod get_descriptor;

use bdk::{
    bitcoin::{secp256k1::Secp256k1, Amount, Network},
    bitcoincore_rpc::{Auth as rpc_auth, Client, RpcApi},
    blockchain::{
        rpc::{wallet_name_from_descriptor, Auth, RpcBlockchain, RpcConfig},
        ConfigurableBlockchain, NoopProgress,
    },
    sled,
    wallet::{signer::SignOptions, AddressIndex},
    Wallet,
};


fn main() {
    let (receive_desc, change_desc) = get_descriptor::get_descriptors();

    println!("recv: {:#?}, \nchange: {:#?}", receive_desc, change_desc);

    // create an rpc interface
    let rpc_auth = rpc_auth::UserPass("test".to_string(), "test".to_string());
    let core_rpc = Client::new("http://127.0.0.1:10001/wallet/test".to_string(), rpc_auth).unwrap();

    println!("{:#?}", core_rpc.get_blockchain_info().unwrap());

    core_rpc
        .create_wallet("test", None, None, None, None)
        .unwrap();

    let core_address = core_rpc.get_new_address(None, None).unwrap();

    core_rpc.generate_to_address(101, &core_address).unwrap();

    let core_balance = core_rpc.get_balance(None, None);
    println!("core balance: {:#?}", core_balance);

    // Use deterministic wallet name derived from descriptor
    let wallet_name = wallet_name_from_descriptor(
        &receive_desc,
        Some(&change_desc),
        Network::Regtest,
        &Secp256k1::new(),
    )
    .unwrap();

    // Create the datadir to store wallet data
    let mut datadir = dirs_next::home_dir().unwrap();
    datadir.push("bdk.example");
    let database = sled::open(datadir).unwrap();
    let db_tree = database.open_tree(wallet_name.clone()).unwrap();

    // Set RPC username, password and url
    let auth = Auth::UserPass {
        username: "test".to_string(),
        password: "test".to_string(),
    };
    let mut rpc_url = "https://".to_string();
    rpc_url.push_str("127.0.0.1:10001");

    // Setup the RPC configuration
    let rpc_config = RpcConfig {
        url: rpc_url,
        auth,
        network: Network::Regtest,
        wallet_name,
        skip_blocks: None,
    };

    // Use the above configuration to create a RPC blockchain backend
    let blockchain = RpcBlockchain::from_config(&rpc_config).unwrap();

    // Combine everything and finally create the BDK wallet structure
    let wallet = Wallet::new(
        &receive_desc,
        Some(&change_desc),
        Network::Regtest,
        db_tree,
        blockchain,
    )
    .unwrap();

    // sync the wallet
    wallet.sync(NoopProgress, None).unwrap();

    // Fetch a fresh address to receive coins
    let address = wallet.get_address(AddressIndex::New).unwrap().address;
    println!("bdk address: {:#?}", address);

    // Send 10 BTC from Core to BDK
    core_rpc
        .send_to_address(
            &address,
            Amount::from_btc(10.0).unwrap(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    // Confirm transaction by generating some blocks
    core_rpc.generate_to_address(5, &core_address).unwrap();

    // sync the bdk wallet
    wallet.sync(NoopProgress, None).unwrap();

    // create a builder transaction
    let mut tx_builder = wallet.build_tx();

    // set recipient of transaction
    tx_builder.set_recipients(vec![(core_address.script_pubkey(), 500000000)]);

    // finalise the trx and extract the PSBT
    let (mut psbt, _) = tx_builder.finish().unwrap();

    // set signing option
    let signopt = SignOptions {
        assume_height: None,
        ..Default::default()
    };

    // Sign the above psbt with signing option
    wallet.sign(&mut psbt, signopt).unwrap();

    // Extract the final transaction
    let tx = psbt.extract_tx();

    // Broadcast the transaction
    wallet.broadcast(tx).unwrap();

    // Confirm transaction by generating some blocks
    core_rpc.generate_to_address(5, &core_address).unwrap();

    // sync the BDK wallet
    wallet.sync(NoopProgress, None).unwrap();

    // // Fetch and display wallet balances
    let core_balance = core_rpc.get_balance(None, None).unwrap();
    let bdk_balance = Amount::from_sat(wallet.get_balance().unwrap());

    println!("Core balance: {:#?}", core_balance);
    println!("Bdk balance: {:#?}", bdk_balance);
}
