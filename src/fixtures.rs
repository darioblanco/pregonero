use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use store::Store;
use tracing::debug;

use crate::store;

pub async fn load_accounts(store: &Arc<dyn Store>) {
    let accounts_filepath = "accounts.json";
    // Open the JSON file
    let mut file = File::open(accounts_filepath).unwrap();

    // Read the contents of the file into a string
    let mut json_string = String::new();
    file.read_to_string(&mut json_string).unwrap();

    // Deserialize the JSON string into a Vec<MyStruct>
    let accounts: Vec<store::Account> = serde_json::from_str(&json_string).unwrap();
    for account in accounts {
        debug!(
            "Processing account: {:?} from file '{}'",
            account, accounts_filepath
        );
        store.store_account(account).await.unwrap();
    }
}
