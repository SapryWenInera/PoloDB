use std::io::Write;
use byteorder::WriteBytesExt;
use libc::printf;
use crate::lsm::mem_table::MemTable;
use crate::{DbErr, DbResult, TransactionType};
use crate::lsm::KvCursor;
use crate::lsm::lsm_backend::lsm_log::format;
use crate::lsm::multi_cursor::MultiCursor;
use crate::utils::vli;

pub struct LsmSession {
    id: u64,
    pub(crate) mem_table: MemTable,
    log_buffer: Option<Vec<u8>>,
    transaction: Option<TransactionType>,
}

impl LsmSession {

    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn new(id: u64, mem_table: MemTable, has_log: bool) -> LsmSession {
        let log_buffer = if has_log {
            Some(Vec::new())
        } else {
            None
        };
        LsmSession {
            id,
            mem_table,
            log_buffer,
            transaction: None,
        }
    }

    #[inline]
    pub fn log_buffer(&self) -> Option<&[u8]> {
        self.log_buffer.as_ref().map(|buf| buf.as_slice())
    }

    pub fn transaction(&self) -> Option<TransactionType> {
        self.transaction
    }

    pub fn start_transaction(&mut self, ty: TransactionType) -> DbResult<()> {
        if self.transaction.is_some() {
            return Err(DbErr::StartTransactionInAnotherTransaction);
        }
        self.transaction = Some(ty);
        Ok(())
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) -> DbResult<()> {
        if let Some(log_buffer) = &mut self.log_buffer {
            LsmSession::put_log(log_buffer, key, value)?;
        }

        self.mem_table.put(key, value, false);

        Ok(())
    }

    fn put_log<W: Write>(writer: &mut W, key: &[u8], value: &[u8]) -> DbResult<()> {
        writer.write_u8(format::WRITE)?;

        let key_len = key.len();
        vli::encode(writer, key_len as i64)?;

        writer.write_all(key)?;

        let value_len = value.len();
        vli::encode(writer, value_len as i64)?;

        writer.write_all(value)?;

        Ok(())
    }

    pub fn delete(&mut self, key: &[u8]) -> DbResult<()> {
        if let Some(log_buffer) = &mut self.log_buffer {
            LsmSession::delete_log(log_buffer, key)?;
        }

        self.mem_table.delete(key, false);

        Ok(())
    }

    fn delete_log<W: Write>(writer: &mut W, key: &[u8]) -> DbResult<()> {
        writer.write_u8(format::DELETE)?;

        let key_len = key.len();
        vli::encode(writer, key_len as i64)?;

        writer.write_all(key)?;

        Ok(())
    }

    pub(crate) fn update_cursor_current(&mut self, cursor: &mut MultiCursor, value: &[u8]) -> DbResult<bool> {
        let key = cursor.key();
        if key.is_none() {
            return Ok(false);
        }
        let mut result = false;
        let key = key.as_ref().unwrap();

        if let Some(log_buffer) = &mut self.log_buffer {
            LsmSession::put_log(log_buffer, key, value)?;
        }

        let new_tree_opt = cursor.update_current(value);
        if let Some((new_tree, legacy_value_opt)) = new_tree_opt {
            self.mem_table.update_root(new_tree);

            if let Some(legacy_value) = legacy_value_opt {
                *self.mem_table.store_bytes_mut() -= legacy_value.len();
                result = true;
            }

            *self.mem_table.store_bytes_mut() += value.len();
        }

        Ok(result)
    }

    pub(crate) fn delete_cursor_current(&mut self, cursor: &mut MultiCursor) -> DbResult<bool> {
        let key = cursor.key();
        if key.is_none() {
            return Ok(false);
        }
        let mut result = false;
        let key = key.as_ref().unwrap();

        if let Some(log_buffer) = &mut self.log_buffer {
            LsmSession::delete_log(log_buffer, key)?;
        }

        let new_tree_opt = cursor.delete_current();
        if let Some((new_tree, legacy_value_opt)) = new_tree_opt {
            self.mem_table.update_root(new_tree);

            if let Some(legacy_value) = legacy_value_opt {
                // The "key" and "mark" still needs space
                // only substract the space of value here
                *self.mem_table.store_bytes_mut() -= legacy_value.len();
                result = true;
            }
        }

        Ok(result)
    }

    pub(crate) fn finished_transaction(&mut self) {
        if self.log_buffer.is_some() {
            self.log_buffer = Some(Vec::new());
        }
        self.transaction = None;
        self.id += 1;
    }

}
