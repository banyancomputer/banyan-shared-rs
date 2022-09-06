use crate::types::{BlockNum, OnChainDealInfo};

#[derive(Debug)]
pub enum DealStatusError {
    Future,
    Past,
}

// TODO: check this it might not be correct. write some tests. from copilot. suspect this is wrong.
pub fn get_the_current_window(
    deal_info: &OnChainDealInfo,
    current_block_num: BlockNum,
) -> Result<BlockNum, DealStatusError> {
    if current_block_num < deal_info.deal_start_block {
        return Err(DealStatusError::Future);
    };
    if current_block_num >= deal_info.deal_start_block + deal_info.deal_length_in_blocks {
        return Err(DealStatusError::Past);
    };
    // this should return the window start of the current block
    let window_number = (current_block_num - deal_info.deal_start_block)
        .0
        .div_euclid(deal_info.proof_frequency_in_blocks.0);
    Ok(deal_info.deal_start_block + deal_info.proof_frequency_in_blocks * window_number)
}

// TODO: check this it might not be correct. write some tests. from copilot. suspect this is wrong.
/// Some(n) where n is the next window start block, or None if the deal is complete.
pub fn get_the_next_window(
    deal_info: &OnChainDealInfo,
    last_submission: BlockNum,
) -> Option<BlockNum> {
    let window_number = (last_submission - deal_info.deal_start_block)
        .0
        .div_euclid(deal_info.proof_frequency_in_blocks.0);
    let elapsed_length = deal_info.proof_frequency_in_blocks * (window_number + 1);
    if elapsed_length >= deal_info.deal_length_in_blocks {
        None
    } else {
        Some(deal_info.deal_start_block + elapsed_length)
    }
}

// tests
#[cfg(test)]
mod tests {
    // in this test, we have a deal with length 21, proof window of size 5, and start block is 3

    #[test]
    fn get_the_right_window_works() {
        unimplemented!("fail");
    }
    #[test]
    fn get_the_next_window_works() {
        unimplemented!("fail");
    }
}
