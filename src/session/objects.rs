use crate::database::EntityId;

use crate::database::entities::{Revision, IndexInEntity};
use crate::database::update::{Update, UpdateType};
use crate::database::objects::{
    Message, Cell, Element,
    File, ConversationId, SheetId, DocumentId, BucketId, Stamp,
};

use crate::DATABASE;
use super::upload::TemporaryFile;
use super::requests::MessageCursor;
use super::replies::{Reply, ReplyData};

use super::{Session, ErrMsg};

use std::time::{SystemTime, Duration};
use std::mem::{drop, replace};

fn now_stamp() -> Stamp {
    SystemTime::UNIX_EPOCH.elapsed().unwrap_or(Duration::ZERO).as_secs()
}

impl Session {
    pub(super) async fn handle_load_messages_before(
        &mut self,
        num: usize,
        conv_id: ConversationId,
        cursor: MessageCursor,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        arc_user.check_access_to(EntityId::Conversation(conv_id), false).await?;

        let arc_conv = DATABASE.conversations.find(conv_id).await.ok_or("No such conversation")?;
        let conv = arc_conv.read().await;

        let max = conv.messages.len();
        let stop = match cursor {
            MessageCursor::Specific(index) => index as usize,
            MessageCursor::Latest => max,
        };

        let start = stop.saturating_sub(50);
        let Some(slice) = conv.messages.get(start..stop) else {
            return Err("Invalid cursor");
        };

        Ok(Reply::new(num, ReplyData::Messages(conv.revision, slice.to_vec())))
    }

    pub(super) async fn handle_post_message(
        &mut self,
        num: usize,
        conv_id: ConversationId,
        rev: Revision,
        content: String,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, user_id) = self.get_user().await?;
        arc_user.check_access_to(EntityId::Conversation(conv_id), true).await?;

        let message = Message {
            author: user_id,
            content,
            created: now_stamp(),
            edited: None,
        };

        let arc_conv = DATABASE.conversations.find(conv_id).await.ok_or("No such conversation")?;
        let mut conv = arc_conv.write().await;

        if conv.revision != rev {
            return Err("Out of date");
        }

        let index = conv.messages.len() as u64;
        let update = Update::message(conv_id, conv.revision, index, &message);
        conv.messages.push(message);

        drop(conv);
        DATABASE.notify_users(update).await;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_edit_message(
        &mut self,
        num: usize,
        conv_id: ConversationId,
        rev: Revision,
        index: IndexInEntity,
        new_content: String,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        arc_user.check_access_to(EntityId::Conversation(conv_id), true).await?;

        let arc_conv = DATABASE.conversations.find(conv_id).await.ok_or("No such conversation")?;
        let mut conv = arc_conv.write().await;
        let bad_index = index >= (conv.messages.len() as u64);
        if conv.revision != rev || bad_index {
            return Err("Out of date or bad index");
        }

        let rev = conv.revision;
        let message = &mut conv.messages[index as usize];
        message.content = new_content;
        message.edited = Some(now_stamp());
        let update = Update::message(conv_id, rev, index, message);

        drop(conv);
        DATABASE.notify_users(update).await;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_load_spreadsheet(
        &mut self,
        num: usize,
        sheet: SheetId,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        arc_user.check_access_to(EntityId::Spreadsheet(sheet), false).await?;

        let arc_sheet = DATABASE.sheets.find(sheet).await.ok_or("No such spreadsheet")?;
        let sheet = arc_sheet.read().await;
        let cells = sheet.cells.as_slice().to_vec();

        Ok(Reply::new(num, ReplyData::Spreadsheet(sheet.revision, cells)))
    }

    pub(super) async fn handle_set_cell(
        &mut self,
        num: usize,
        sheet_id: SheetId,
        rev: Revision,
        index: IndexInEntity,
        cell: Cell,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        arc_user.check_access_to(EntityId::Spreadsheet(sheet_id), true).await?;

        let arc_sheet = DATABASE.sheets.find(sheet_id).await.ok_or("No such spreadsheet")?;
        let mut sheet = arc_sheet.write().await;

        if sheet.revision != rev {
            return Err("Out of date");
        }

        let update = Update::cell(sheet_id, sheet.revision, index, &cell);
        sheet.cells[&index] = cell;

        drop(sheet);
        DATABASE.notify_users(update).await;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_load_document(
        &mut self,
        num: usize,
        doc: DocumentId,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        arc_user.check_access_to(EntityId::Document(doc), false).await?;

        let arc_doc = DATABASE.documents.find(doc).await.ok_or("No such document")?;
        let doc = arc_doc.read().await;
        let elements = doc.elements.to_vec();
        let rev = doc.revision;

        Ok(Reply::new(num, ReplyData::Document(rev, elements)))
    }

    pub(super) async fn handle_insert_element(
        &mut self,
        num: usize,
        doc_id: DocumentId,
        rev: Revision,
        index: IndexInEntity,
        element: Element,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        let entity_id = EntityId::Document(doc_id);
        arc_user.check_access_to(entity_id, true).await?;
        let upd_type = UpdateType::NewElement;

        let arc_doc = DATABASE.documents.find(doc_id).await.ok_or("No such document")?;
        let mut doc = arc_doc.write().await;
        let bad_rev = doc.revision != rev;
        let bad_index = index > (doc.elements.len() as u64);

        if bad_rev || bad_index {
            return Err("Out of date or bad index");
        }

        doc.revision += 1;
        let update = Update::new(upd_type, entity_id, doc.revision, index, &element);
        doc.elements.insert(index as usize, element);

        drop(doc);
        DATABASE.notify_users(update).await;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_delete_element(
        &mut self,
        num: usize,
        doc_id: DocumentId,
        rev: Revision,
        index: IndexInEntity,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        let entity_id = EntityId::Document(doc_id);
        arc_user.check_access_to(entity_id, true).await?;
        let upd_type = UpdateType::ByeElement;

        let arc_doc = DATABASE.documents.find(doc_id).await.ok_or("No such document")?;
        let mut doc = arc_doc.write().await;
        let bad_rev = doc.revision != rev;
        let bad_index = index >= (doc.elements.len() as u64);

        if bad_rev || bad_index {
            return Err("Out of date or bad index");
        }

        doc.revision += 1;
        let update = Update::new(upd_type, entity_id, doc.revision, index, &"");
        doc.elements.remove(index as usize);

        drop(doc);
        DATABASE.notify_users(update).await;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_set_element(
        &mut self,
        num: usize,
        doc_id: DocumentId,
        rev: Revision,
        index: IndexInEntity,
        element: Element,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        let entity_id = EntityId::Document(doc_id);
        arc_user.check_access_to(entity_id, true).await?;
        let upd_type = UpdateType::SetElement;

        let arc_doc = DATABASE.documents.find(doc_id).await.ok_or("No such document")?;
        let mut doc = arc_doc.write().await;
        let bad_rev = doc.revision != rev;
        let bad_index = index >= (doc.elements.len() as u64);

        if bad_rev || bad_index {
            return Err("Out of date or bad index");
        }

        doc.revision += 1;
        let update = Update::new(upd_type, entity_id, doc.revision, index, &element);
        doc.elements[index as usize] = element;

        drop(doc);
        DATABASE.notify_users(update).await;
        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_load_bucket(
        &mut self,
        num: usize,
        bucket_id: BucketId,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        arc_user.check_access_to(EntityId::Bucket(bucket_id), false).await?;

        let arc_bucket = DATABASE.buckets.find(bucket_id).await.ok_or("No such bucket")?;
        let bucket = arc_bucket.read().await;
        let files = bucket.files.to_vec();
        let rev = bucket.revision;

        Ok(Reply::new(num, ReplyData::Bucket(rev, files)))
    }

    pub(super) async fn handle_delete_file(
        &mut self,
        num: usize,
        bucket_id: BucketId,
        rev: Revision,
        index: IndexInEntity,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        let entity_id = EntityId::Bucket(bucket_id);
        arc_user.check_access_to(entity_id, true).await?;
        let upd_type = UpdateType::ByeFile;

        let arc_bucket = DATABASE.buckets.find(bucket_id).await.ok_or("No such bucket")?;
        let mut bucket = arc_bucket.write().await;
        let bad_rev = bucket.revision != rev;
        let bad_index = index >= (bucket.files.len() as u64);

        if bad_rev || bad_index {
            return Err("Out of date or bad index");
        }

        bucket.revision += 1;
        let update = Update::new(upd_type, entity_id, bucket.revision, index, &"");
        let file = bucket.files.remove(index as usize);

        drop(bucket);
        DATABASE.notify_users(update).await;
        DATABASE.dec_file_rc(&file.sha256).await;

        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_set_file(
        &mut self,
        num: usize,
        bucket_id: BucketId,
        rev: Revision,
        index: Option<IndexInEntity>,
        file: File,
    ) -> Result<Reply, ErrMsg> {
        let (arc_user, _user_id) = self.get_user().await?;
        let entity_id = EntityId::Bucket(bucket_id);
        arc_user.check_access_to(entity_id, true).await?;

        let arc_bucket = DATABASE.buckets.find(bucket_id).await.ok_or("No such bucket")?;
        let mut bucket = arc_bucket.write().await;
        let len = bucket.files.len() as u64;
        let index = index.unwrap_or(len);
        let diff = len.checked_sub(index).ok_or("Bad index")?;
        let bad_rev = bucket.revision != rev;
        let new_file = diff == 0;

        if bad_rev {
            return Err("Out of date");
        }

        let upd_type = match new_file {
            true => UpdateType::NewFile,
            false => UpdateType::SetFile,
        };

        bucket.revision += 1;
        let update = Update::new(upd_type, entity_id, bucket.revision, index, &file);
        DATABASE.inc_file_rc(&file.sha256).await;

        let maybe_old_file = if new_file {
            bucket.files.push(file);
            None
        } else {
            let mem_loc = &mut bucket.files[index as usize];
            Some(replace(mem_loc, file))
        };

        drop(bucket);
        DATABASE.notify_users(update).await;

        if let Some(old_file) = maybe_old_file {
            DATABASE.dec_file_rc(&old_file.sha256).await;
        }

        Ok(Reply::new(num, ReplyData::GenericSuccess))
    }

    pub(super) async fn handle_finish_file(
        &mut self,
        num: usize,
        bucket_id: BucketId,
        rev: Revision,
        name: String,
    ) -> Result<Reply, ErrMsg> {
        self.user_id.ok_or("Not logged in yet")?;

        let tmp_file = match self.tmp_file.take() {
            Some(tmp_file) => tmp_file,
            None => TemporaryFile::new().await?,
        };

        let (sha256, size) = tmp_file.finalize().await;
        let file_data = File {
            name,
            sha256,
            size,
            uploaded: now_stamp(),
        };

        self.handle_set_file(num, bucket_id, rev, None, file_data).await
    }
}