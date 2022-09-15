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

// should be right, haven't tested extensively
pub fn get_num_windows(
    deal_length: BlockNum,
    window_size: BlockNum,
) -> Result<usize, anyhow::Error> {
    if window_size.0 == 0 {
        return Err(anyhow::anyhow!("Cannot divide by zero"));
    }
    Ok(math::round::ceil((deal_length.0 / window_size.0) as f64, 0) as usize)
}

// tests
#[cfg(test)]
mod tests {

    // in this test, we have a deal with length 21, proof window of size 5, and start block is 3
    use super::*;
    #[test]
    fn get_num_windows_works() {
        let (deal_length1, window_size1) = (BlockNum(20), BlockNum(2));
        let (deal_length2, window_size2) = (BlockNum(20), BlockNum(3));
        //let (deal_length3, window_size3) = (BlockNum(20), BlockNum(0));
        assert_eq!(get_num_windows(deal_length1, window_size1).unwrap(), 10);
        assert_eq!(get_num_windows(deal_length2, window_size2).unwrap(), 6);
        //couldn't figure out how to get the error test working
    }
}
