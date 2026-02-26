use primitives::{AccountID, Currency, UInt256};
use serialization::{Amount, PathStep};

/// Offer represents a DEX order
#[derive(Debug, Clone)]
pub struct Offer {
    pub account: AccountID,
    pub sequence: u32,
    pub taker_gets: Amount,
    pub taker_pays: Amount,
    pub rate: f64,
}

impl Offer {
    pub fn new(account: AccountID, sequence: u32, taker_gets: Amount, taker_pays: Amount) -> Self {
        let rate = calculate_rate(taker_pays, taker_gets);
        Self {
            account,
            sequence,
            taker_gets,
            taker_pays,
            rate,
        }
    }

    pub fn get_quality(&self) -> f64 {
        self.rate
    }

    pub fn matches(&self, other: &Offer) -> bool {
        // Check if two offers can cross
        self.taker_gets.get_currency() == other.taker_pays.get_currency()
            && self.taker_pays.get_currency() == other.taker_gets.get_currency()
    }
}

/// Calculate exchange rate between two amounts
fn calculate_rate(pays: Amount, gets: Amount) -> f64 {
    if gets.is_zero() {
        return 0.0;
    }

    let pays_val = pays.mantissa as f64 * 10f64.powi(pays.exponent);
    let gets_val = gets.mantissa as f64 * 10f64.powi(gets.exponent);

    pays_val / gets_val
}

/// OfferBook represents all offers for a currency pair
#[derive(Debug, Clone)]
pub struct OfferBook {
    pub taker_gets_currency: Currency,
    pub taker_pays_currency: Currency,
    pub offers: Vec<Offer>,
}

impl OfferBook {
    pub fn new(
        taker_gets_currency: Currency,
        taker_pays_currency: Currency,
    ) -> Self {
        Self {
            taker_gets_currency,
            taker_pays_currency,
            offers: Vec::new(),
        }
    }

    pub fn add_offer(&mut self, offer: Offer) {
        self.offers.push(offer);
        // Sort by quality (best rate first)
        self.offers.sort_by(|a, b| {
            a.get_quality()
                .partial_cmp(&b.get_quality())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn remove_offer(&mut self, account: AccountID, sequence: u32) -> bool {
        if let Some(pos) = self
            .offers
            .iter()
            .position(|o| o.account == account && o.sequence == sequence)
        {
            self.offers.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_best_offer(&self) -> Option<&Offer> {
        self.offers.first()
    }
}

/// BookKey identifies an offer book
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BookKey {
    pub pays_currency: Currency,
    pub gets_currency: Currency,
}

impl BookKey {
    pub fn new(pays: Currency, gets: Currency) -> Self {
        Self {
            pays_currency: pays,
            gets_currency: gets,
        }
    }
}

/// Taker represents a party taking offers from the book
#[derive(Debug)]
pub struct Taker {
    pub account: AccountID,
    pub max_to_spend: Amount,
    pub min_to_receive: Amount,
}

impl Taker {
    pub fn new(account: AccountID, max_to_spend: Amount, min_to_receive: Amount) -> Self {
        Self {
            account,
            max_to_spend,
            min_to_receive,
        }
    }
}

/// Flow represents the result of a payment flow calculation
#[derive(Debug, Clone)]
pub struct Flow {
    pub source_amount: Amount,
    pub destination_amount: Amount,
    pub path: Vec<PathStep>,
}

impl Flow {
    pub fn new(source: Amount, destination: Amount) -> Self {
        Self {
            source_amount: source,
            destination_amount: destination,
            path: Vec::new(),
        }
    }
}

/// Pathfinder finds paths for payments
pub struct Pathfinder {
    max_search_depth: usize,
}

impl Pathfinder {
    pub fn new() -> Self {
        Self {
            max_search_depth: 7,
        }
    }

    /// Find paths from source to destination
    pub fn find_paths(
        &self,
        _source: AccountID,
        _destination: AccountID,
        _destination_amount: Amount,
    ) -> Vec<Vec<PathStep>> {
        // Placeholder: return direct path
        vec![vec![]]
    }

    /// Find paths with source amount limit
    pub fn find_paths_with_source(
        &self,
        _source: AccountID,
        _destination: AccountID,
        _source_amount: Amount,
    ) -> Vec<Vec<PathStep>> {
        // Placeholder: return direct path
        vec![vec![]]
    }
}

impl Default for Pathfinder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offer_quality() {
        let account = AccountID::new([0u8; 20]);
        let currency1 = Currency::CALL;
        let currency2 = Currency::new([1u8; 20]);

        let taker_gets = Amount::call(1000000);
        let taker_pays = Amount::issued(2000000, -6, currency2, account).unwrap();

        let offer = Offer::new(account, 1, taker_gets, taker_pays);
        assert!(offer.get_quality() > 0.0);
    }

    #[test]
    fn test_offer_book() {
        let currency1 = Currency::CALL;
        let currency2 = Currency::new([1u8; 20]);

        let mut book = OfferBook::new(currency1, currency2);
        assert!(book.get_best_offer().is_none());

        let account = AccountID::new([0u8; 20]);
        let taker_gets = Amount::call(1000000);
        let taker_pays = Amount::issued(2000000, -6, currency2, account).unwrap();

        let offer = Offer::new(account, 1, taker_gets, taker_pays);
        book.add_offer(offer);

        assert!(book.get_best_offer().is_some());
    }
}
