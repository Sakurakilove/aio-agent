use super::compressor::MessageWithTokens;

pub struct StreamingContextScrubber {
    max_messages: usize,
    preserve_last_n: usize,
}

impl StreamingContextScrubber {
    pub fn new(max_messages: usize, preserve_last_n: usize) -> Self {
        Self {
            max_messages,
            preserve_last_n,
        }
    }

    pub fn scrub(&self, messages: &mut Vec<MessageWithTokens>) {
        if messages.len() > self.max_messages {
            let keep_count = self.preserve_last_n.min(messages.len());
            let remove_count = messages.len() - keep_count;
            messages.drain(..remove_count);
        }
    }

    pub fn add_message(&self, messages: &mut Vec<MessageWithTokens>, message: MessageWithTokens) {
        messages.push(message);
        self.scrub(messages);
    }
}
