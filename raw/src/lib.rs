// BP Core Library implementing LNP/BP specifications & standards related to
// bitcoin protocol
//
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the Apache 2.0 License
// along with this software.
// If not, see <https://opensource.org/licenses/Apache-2.0>.

#[macro_use]
extern crate amplify;

mod serialize;
mod sha256;
mod tx;

use amplify::confinement::Confined;
pub use sha256::Sha256;
pub use tx::{
    LockTime, Sats, ScriptPubkey, SeqNo, SigScript, Tx, TxIn, TxOut, TxVer,
    Txid,
};

pub type VarIntArray<T> = Confined<Vec<T>, 0, { u64::MAX as usize }>;
pub type VarIntBytes = VarIntArray<u8>;
