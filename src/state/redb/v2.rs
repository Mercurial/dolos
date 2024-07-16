use ::redb::{Database, Durability, Error};
use std::sync::Arc;

use crate::ledger::*;

use super::tables;

#[derive(Clone)]
pub struct LedgerStore(pub Arc<Database>);

impl LedgerStore {
    pub fn initialize(db: Database) -> Result<Self, Error> {
        let mut wx = db.begin_write()?;
        wx.set_durability(Durability::Immediate);

        tables::CursorTable::initialize(&wx)?;
        tables::UtxosTable::initialize(&wx)?;
        tables::PParamsTable::initialize(&wx)?;
        tables::FilterIndexes::initialize(&wx)?;

        wx.commit()?;

        Ok(db.into())
    }

    pub fn is_empty(&self) -> Result<bool, Error> {
        self.cursor().map(|x| x.is_none())
    }

    pub fn cursor(&self) -> Result<Option<ChainPoint>, Error> {
        let rx = self.0.begin_read()?;

        let last = tables::CursorTable::last(&rx)?.map(|(k, v)| ChainPoint(k, v.hash));

        Ok(last)
    }

    pub fn apply(&mut self, deltas: &[LedgerDelta]) -> Result<(), Error> {
        let mut wx = self.0.begin_write()?;
        wx.set_durability(Durability::Eventual);

        for delta in deltas {
            tables::CursorTable::apply(&wx, delta)?;
            tables::UtxosTable::apply(&wx, delta)?;
            tables::PParamsTable::apply(&wx, delta)?;
            tables::FilterIndexes::apply(&wx, delta)?;
        }

        wx.commit()?;

        Ok(())
    }

    pub fn finalize(&mut self, until: BlockSlot) -> Result<(), Error> {
        let rx = self.0.begin_read()?;
        let cursors = tables::CursorTable::get_range(&rx, until)?;

        let mut wx = self.0.begin_write()?;
        wx.set_durability(Durability::Eventual);

        for (slot, value) in cursors {
            tables::CursorTable::compact(&wx, slot)?;
            tables::UtxosTable::compact(&wx, slot, &value.tombstones)?;
        }

        wx.commit()?;

        Ok(())
    }

    pub fn get_utxos(&self, refs: Vec<TxoRef>) -> Result<UtxoMap, Error> {
        // exit early before opening a read tx in case there's nothing to fetch
        if refs.is_empty() {
            return Ok(Default::default());
        }

        let rx = self.0.begin_read()?;
        tables::UtxosTable::get_sparse(&rx, refs)
    }

    pub fn get_pparams(&self, until: BlockSlot) -> Result<Vec<PParamsBody>, Error> {
        let rx = self.0.begin_read()?;
        tables::PParamsTable::get_range(&rx, until)
    }

    pub fn get_utxos_by_address(&self, address: &[u8]) -> Result<UtxoSet, Error> {
        let rx = self.0.begin_read()?;
        tables::FilterIndexes::get_by_address(&rx, address)
    }

    pub fn get_utxos_by_payment(&self, payment: &[u8]) -> Result<UtxoSet, Error> {
        let rx = self.0.begin_read()?;
        tables::FilterIndexes::get_by_payment(&rx, payment)
    }

    pub fn get_utxos_by_stake(&self, stake: &[u8]) -> Result<UtxoSet, Error> {
        let rx = self.0.begin_read()?;
        tables::FilterIndexes::get_by_stake(&rx, stake)
    }

    pub fn get_utxos_by_policy(&self, policy: &[u8]) -> Result<UtxoSet, Error> {
        let rx = self.0.begin_read()?;
        tables::FilterIndexes::get_by_policy(&rx, policy)
    }

    pub fn get_utxos_by_asset(&self, asset: &[u8]) -> Result<UtxoSet, Error> {
        let rx = self.0.begin_read()?;
        tables::FilterIndexes::get_by_asset(&rx, asset)
    }
}

impl From<Database> for LedgerStore {
    fn from(value: Database) -> Self {
        Self(Arc::new(value))
    }
}

#[cfg(test)]
mod tests {
    use pallas::crypto::hash::Hash;

    use super::*;

    #[test]
    fn empty_until_cursor() {
        let db = ::redb::Database::builder()
            .create_with_backend(::redb::backends::InMemoryBackend::new())
            .unwrap();

        let mut store = LedgerStore::initialize(db).unwrap();
        assert!(store.is_empty().unwrap());

        let delta = LedgerDelta {
            new_position: Some(ChainPoint(
                1,
                Hash::new(b"01010101010101010101010101010101".to_owned()),
            )),
            undone_position: Default::default(),
            produced_utxo: Default::default(),
            consumed_utxo: Default::default(),
            recovered_stxi: Default::default(),
            undone_utxo: Default::default(),
            new_pparams: Default::default(),
        };

        store.apply(&[delta]).unwrap();
        assert!(!store.is_empty().unwrap());
    }
}