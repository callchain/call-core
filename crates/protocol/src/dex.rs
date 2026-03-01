use primitives::{AccountID, Currency};
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


/// Trust line for path finding
#[derive(Debug, Clone)]
pub struct TrustLine {
    pub account: AccountID,
    pub issuer: AccountID,
    pub currency: Currency,
    pub balance: Amount,
    pub limit: Amount,
    /// QualityIn: How much the account values incoming IOUs (0 = default 1.0)
    pub quality_in: Option<u32>,
    /// QualityOut: How much the account values outgoing IOUs (0 = default 1.0)
    pub quality_out: Option<u32>,
}

impl TrustLine {
    pub fn new(account: AccountID, issuer: AccountID, currency: Currency, limit: Amount) -> Self {
        Self {
            account,
            issuer,
            currency,
            balance: Amount::issued(0, 0, currency, issuer).unwrap_or_else(|| Amount::call(0)),
            limit,
            quality_in: None,
            quality_out: None,
        }
    }

    /// Create a trust line with quality settings
    pub fn with_quality(
        account: AccountID,
        issuer: AccountID,
        currency: Currency,
        limit: Amount,
        quality_in: Option<u32>,
        quality_out: Option<u32>,
    ) -> Self {
        Self {
            account,
            issuer,
            currency,
            balance: Amount::issued(0, 0, currency, issuer).unwrap_or_else(|| Amount::call(0)),
            limit,
            quality_in,
            quality_out,
        }
    }

    /// Get the quality multiplier for incoming IOUs
    /// Returns 1.0 if no quality is set (default behavior)
    /// Quality is stored as a 32-bit unsigned integer where 1000000000 = 1.0
    pub fn get_quality_in(&self) -> f64 {
        self.quality_in
            .map(|q| if q == 0 { 1.0 } else { q as f64 / 1_000_000_000.0 })
            .unwrap_or(1.0)
    }

    /// Get the quality multiplier for outgoing IOUs
    pub fn get_quality_out(&self) -> f64 {
        self.quality_out
            .map(|q| if q == 0 { 1.0 } else { q as f64 / 1_000_000_000.0 })
            .unwrap_or(1.0)
    }

    /// Check if this trust line can send the specified amount
    pub fn can_send(&self, amount: &Amount) -> bool {
        // Can send if balance >= amount (for positive balances)
        self.balance.mantissa >= amount.mantissa
    }

    /// Check if this trust line can receive the specified amount
    pub fn can_receive(&self, amount: &Amount) -> bool {
        // Can receive if balance + amount <= limit
        let new_balance = self.balance.mantissa.saturating_add(amount.mantissa);
        new_balance <= self.limit.mantissa
    }

    /// Apply quality in to an amount
    /// Returns the effective amount received after quality adjustment
    pub fn apply_quality_in(&self, amount: u64) -> u64 {
        ((amount as f64) * self.get_quality_in()) as u64
    }

    /// Apply quality out to an amount
    /// Returns the effective amount sent after quality adjustment
    pub fn apply_quality_out(&self, amount: u64) -> u64 {
        ((amount as f64) / self.get_quality_out()) as u64
    }
}

/// A found path with cost information
#[derive(Debug, Clone)]
pub struct FoundPath {
    /// The path steps
    pub steps: Vec<PathStep>,
    /// Source amount required
    pub source_amount: Amount,
    /// Destination amount that will be received
    pub destination_amount: Amount,
    /// Quality of the path (destination / source ratio)
    pub quality: f64,
}

/// Path cost calculator for computing actual amounts through paths
pub struct PathCostCalculator<'a> {
    pathfinder: &'a Pathfinder,
}

/// Represents a node in the expanded pathfinding graph with amount tracking
#[derive(Debug, Clone)]
struct PathNodeWithAmount {
    account: AccountID,
    currency: Currency,
    /// Amount that can be delivered to this node
    deliverable_amount: Amount,
    /// Amount that needs to be sent from source
    source_cost: Amount,
    path: Vec<PathStep>,
    depth: usize,
}

impl<'a> PathCostCalculator<'a> {
    pub fn new(pathfinder: &'a Pathfinder) -> Self {
        Self { pathfinder }
    }

    /// Calculate the output amount for a given input through an offer book
    fn calculate_offer_book_output(
        &self,
        input_amount: Amount,
        book_key: &BookKey,
    ) -> Option<Amount> {
        let book = self.pathfinder.offer_books.get(book_key)?;
        let mut remaining_input = input_amount.mantissa.max(0) as u64;
        let mut total_output: u64 = 0;
        let output_currency = book_key.gets_currency;

        for offer in &book.offers {
            if remaining_input == 0 {
                break;
            }

            // How much can this offer provide?
            let offer_gets = offer.taker_gets.mantissa.max(0) as u64;
            let offer_pays = offer.taker_pays.mantissa.max(0) as u64;

            if offer_gets == 0 || offer_pays == 0 {
                continue;
            }

            // Rate = pays / gets (what we receive / what we give)
            let rate = offer_pays as f64 / offer_gets as f64;

            // Take the minimum of what we need and what the offer can give
            let take_from_offer = remaining_input.min(offer_gets);
            let output_from_offer = (take_from_offer as f64 * rate) as u64;

            total_output += output_from_offer;
            remaining_input -= take_from_offer;
        }

        if total_output > 0 {
            Amount::issued(total_output as i64, 0, output_currency, AccountID::new([0u8; 20]))
        } else {
            None
        }
    }

    /// Calculate the input amount needed for a desired output through an offer book
    fn calculate_offer_book_input_for_output(
        &self,
        desired_output: Amount,
        book_key: &BookKey,
    ) -> Option<Amount> {
        let book = self.pathfinder.offer_books.get(book_key)?;
        let mut remaining_output = desired_output.mantissa.max(0) as u64;
        let mut total_input: u64 = 0;
        let input_currency = book_key.pays_currency;

        for offer in &book.offers {
            if remaining_output == 0 {
                break;
            }

            let offer_gets = offer.taker_gets.mantissa.max(0) as u64;
            let offer_pays = offer.taker_pays.mantissa.max(0) as u64;

            if offer_gets == 0 || offer_pays == 0 {
                continue;
            }

            // Rate = gets / pays (what we give / what we receive)
            let rate = offer_gets as f64 / offer_pays as f64;

            // How much can we get from this offer?
            let available_from_offer = offer_pays;
            let take_from_offer = remaining_output.min(available_from_offer);
            let input_for_offer = (take_from_offer as f64 * rate) as u64;

            total_input += input_for_offer;
            remaining_output -= take_from_offer;
        }

        if total_input > 0 {
            Amount::issued(total_input as i64, 0, input_currency, AccountID::new([0u8; 20]))
        } else {
            None
        }
    }

    /// Calculate flow through a trust line with quality settings
    fn calculate_trust_line_flow(
        &self,
        amount: Amount,
        from: AccountID,
        to: AccountID,
        currency: Currency,
    ) -> Option<Amount> {
        let trust_line = self.pathfinder.trust_lines.get(&(from, currency, to))?;

        // Check if the sender can actually send this amount
        if !trust_line.can_send(&amount) {
            // Limited by what they can send
            let max_send = trust_line.balance.mantissa.max(0) as u64;
            if max_send == 0 {
                return None;
            }
        }

        // Apply quality out from sender's perspective
        let effective_amount = if let Some(quality_out) = trust_line.quality_out {
            if quality_out > 0 {
                (amount.mantissa.max(0) as u64 * 1_000_000_000) / quality_out as u64
            } else {
                amount.mantissa.max(0) as u64
            }
        } else {
            amount.mantissa.max(0) as u64
        };

        // Check if receiver can receive
        if let Some(amount_check) = Amount::issued(effective_amount as i64, 0, currency, to) {
            if !trust_line.can_receive(&amount_check) {
                return None;
            }
        } else {
            return None;
        }

        Amount::issued(effective_amount as i64, 0, currency, to)
    }
}

/// Pathfinder finds paths for payments
pub struct Pathfinder {
    max_search_depth: usize,
    /// Available offer books (currency pairs)
    offer_books: std::collections::HashMap<BookKey, OfferBook>,
    /// Trust lines indexed by (account, currency, issuer)
    trust_lines: std::collections::HashMap<(AccountID, Currency, AccountID), TrustLine>,
    /// Trust lines by account for quick lookup
    account_trust_lines: std::collections::HashMap<AccountID, Vec<(Currency, AccountID)>>,
}

impl Pathfinder {
    pub fn new() -> Self {
        Self {
            max_search_depth: 6,
            offer_books: std::collections::HashMap::new(),
            trust_lines: std::collections::HashMap::new(),
            account_trust_lines: std::collections::HashMap::new(),
        }
    }

    /// Add an offer book for pathfinding
    pub fn add_offer_book(&mut self, key: BookKey, book: OfferBook) {
        self.offer_books.insert(key, book);
    }

    /// Add a trust line for pathfinding
    pub fn add_trust_line(&mut self, trust_line: TrustLine) {
        let key = (trust_line.account, trust_line.currency, trust_line.issuer);
        self.trust_lines.insert(key, trust_line.clone());

        // Also index by account for quick lookup
        self.account_trust_lines
            .entry(trust_line.account)
            .or_default()
            .push((trust_line.currency, trust_line.issuer));
    }

    /// Get trust lines for an account
    pub fn get_account_trust_lines(&self, account: AccountID) -> Vec<&TrustLine> {
        self.account_trust_lines
            .get(&account)
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(|(currency, issuer)| {
                        self.trust_lines.get(&(account, *currency, *issuer))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if there's a trust line path between two accounts for a currency
    fn can_send_via_trust_line(
        &self,
        sender: AccountID,
        receiver: AccountID,
        currency: Currency,
        amount: &Amount,
    ) -> bool {
        // Check if sender has a trust line to receiver for this currency
        if let Some(trust_line) = self.trust_lines.get(&(sender, currency, receiver)) {
            return trust_line.can_send(amount);
        }

        // Check if receiver has a trust line to sender for this currency
        // (negative balance means sender owes receiver)
        if let Some(trust_line) = self.trust_lines.get(&(receiver, currency, sender)) {
            // Sender can send if receiver's trust line has negative balance
            // (meaning sender owes receiver)
            return trust_line.balance.mantissa < 0 || trust_line.limit.mantissa >= amount.mantissa;
        }

        false
    }

    /// Find paths from source to destination with proper amount calculations
    /// This is the full multi-hop path finding implementation
    pub fn find_paths(
        &self,
        source: AccountID,
        destination: AccountID,
        destination_amount: Amount,
    ) -> Vec<FoundPath> {
        let mut found_paths: Vec<FoundPath> = Vec::new();
        let target_currency = destination_amount.get_currency();
        let calculator = PathCostCalculator::new(self);

        // Use priority queue (by quality/cost) instead of simple BFS
        // We use a BinaryHeap-like approach with Vec for simplicity
        let mut queue: Vec<PathNodeWithAmount> = Vec::new();

        // Start with source's available currencies
        let source_currencies = self.get_currencies_for_account(source);

        for currency in source_currencies {
            queue.push(PathNodeWithAmount {
                account: source,
                currency,
                deliverable_amount: destination_amount.clone(),
                source_cost: Amount::call(0),
                path: Vec::new(),
                depth: 0,
            });
        }

        // Track visited states to avoid cycles (account, currency)
        let mut visited: std::collections::HashSet<(AccountID, Currency)> = std::collections::HashSet::new();

        while let Some(node) = queue.pop() {
            // Limit search depth (max 6 hops as per XRPL spec)
            if node.depth >= self.max_search_depth {
                continue;
            }

            // Check if we reached destination with correct currency
            if node.account == destination && node.currency == target_currency {
                // Calculate actual source cost by working backwards
                let source_cost = if node.path.is_empty() {
                    destination_amount.clone()
                } else {
                    self.calculate_source_cost(&node.path, &destination_amount, source)
                        .unwrap_or_else(|| destination_amount.clone())
                };

                let quality = if source_cost.mantissa > 0 {
                    destination_amount.mantissa as f64 / source_cost.mantissa as f64
                } else {
                    0.0
                };

                found_paths.push(FoundPath {
                    steps: node.path.clone(),
                    source_amount: source_cost.clone(),
                    destination_amount: destination_amount.clone(),
                    quality,
                });

                if found_paths.len() >= 10 {
                    break;
                }
                continue;
            }

            // Mark as visited
            let visit_key = (node.account, node.currency);
            if visited.contains(&visit_key) {
                continue;
            }
            visited.insert(visit_key);

            // Explore offer books for currency conversion
            self.explore_offer_books(
                &node,
                &mut queue,
                destination,
                target_currency,
                &calculator,
            );

            // Explore trust lines
            self.explore_trust_lines(
                &node,
                &mut queue,
                destination,
                target_currency,
                &calculator,
            );
        }

        // Sort paths by quality (best first)
        found_paths.sort_by(|a, b| {
            b.quality.partial_cmp(&a.quality).unwrap_or(std::cmp::Ordering::Equal)
        });

        // If no paths found and this is a CALL payment, add a direct path
        // In XRPL, you can always send CALL directly to any account
        if found_paths.is_empty() && target_currency == Currency::CALL {
            found_paths.push(FoundPath {
                steps: vec![], // Empty path = direct payment
                source_amount: destination_amount.clone(),
                destination_amount: destination_amount.clone(),
                quality: 1.0,
            });
        }

        found_paths
    }

    /// Get all currencies available to an account
    fn get_currencies_for_account(&self, account: AccountID) -> Vec<Currency> {
        let mut currencies = vec![Currency::CALL]; // Always have native currency

        if let Some(trust_lines) = self.account_trust_lines.get(&account) {
            for (currency, _) in trust_lines {
                if !currencies.contains(currency) {
                    currencies.push(*currency);
                }
            }
        }

        currencies
    }

    /// Explore offer books from current node
    fn explore_offer_books(
        &self,
        node: &PathNodeWithAmount,
        queue: &mut Vec<PathNodeWithAmount>,
        _destination: AccountID,
        _target_currency: Currency,
        _calculator: &PathCostCalculator,
    ) {
        for (book_key, book) in &self.offer_books {
            if book_key.pays_currency == node.currency && !book.offers.is_empty() {
                // Create a path step through this order book
                if let Some(best_offer) = book.get_best_offer() {
                    let mut new_path = node.path.clone();
                    new_path.push(PathStep {
                        account: Some(best_offer.account),
                        currency: Some(book_key.gets_currency),
                        issuer: Some(best_offer.account),
                    });

                    queue.push(PathNodeWithAmount {
                        account: best_offer.account,
                        currency: book_key.gets_currency,
                        deliverable_amount: node.deliverable_amount.clone(),
                        source_cost: node.source_cost.clone(),
                        path: new_path,
                        depth: node.depth + 1,
                    });
                }
            }
        }
    }

    /// Explore trust lines from current node
    fn explore_trust_lines(
        &self,
        node: &PathNodeWithAmount,
        queue: &mut Vec<PathNodeWithAmount>,
        destination: AccountID,
        target_currency: Currency,
        _calculator: &PathCostCalculator,
    ) {
        // Check trust lines where this account is the holder
        if let Some(trust_lines) = self.account_trust_lines.get(&node.account) {
            for (currency, issuer) in trust_lines {
                // Only follow trust lines for the currency we need
                if *currency != node.currency && node.currency != Currency::CALL {
                    continue;
                }

                if let Some(trust_line) = self.trust_lines.get(&(node.account, *currency, *issuer)) {
                    if trust_line.can_send(&node.deliverable_amount) {
                        let mut new_path = node.path.clone();
                        new_path.push(PathStep {
                            account: Some(*issuer),
                            currency: Some(*currency),
                            issuer: Some(*issuer),
                        });

                        queue.push(PathNodeWithAmount {
                            account: *issuer,
                            currency: *currency,
                            deliverable_amount: node.deliverable_amount.clone(),
                            source_cost: node.source_cost.clone(),
                            path: new_path,
                            depth: node.depth + 1,
                        });
                    }
                }
            }
        }

        // Check if destination is directly reachable
        if node.currency == target_currency {
            if self.can_send_via_trust_line(node.account, destination, target_currency, &node.deliverable_amount) {
                let mut final_path = node.path.clone();
                final_path.push(PathStep {
                    account: Some(destination),
                    currency: Some(target_currency),
                    issuer: Some(destination),
                });

                queue.push(PathNodeWithAmount {
                    account: destination,
                    currency: target_currency,
                    deliverable_amount: node.deliverable_amount.clone(),
                    source_cost: node.source_cost.clone(),
                    path: final_path,
                    depth: node.depth + 1,
                });
            }
        }
    }

    /// Calculate the source cost required for a path to deliver destination amount
    fn calculate_source_cost(
        &self,
        path: &[PathStep],
        destination_amount: &Amount,
        source: AccountID,
    ) -> Option<Amount> {
        if path.is_empty() {
            return Some(destination_amount.clone());
        }

        let calculator = PathCostCalculator::new(self);
        let mut current_amount = destination_amount.clone();
        let mut current_currency = destination_amount.get_currency();

        // Work backwards from destination to source
        for step in path.iter().rev() {
            if let Some(step_currency) = step.currency {
                if step_currency == current_currency {
                    // Trust line step
                    if let Some(step_account) = step.account {
                        // Check trust line quality
                        if let Some(trust_line) = self.trust_lines.get(&(step_account, step_currency, source)) {
                            // Apply quality out (reverse)
                            let quality_factor = trust_line.get_quality_out();
                            let new_amount = (current_amount.mantissa.max(0) as f64 / quality_factor) as i64;
                            if let Some(new_amt) = Amount::issued(new_amount, 0, step_currency, source) {
                                current_amount = new_amt;
                            }
                        }
                    }
                } else {
                    // Order book step
                    let book_key = BookKey::new(step_currency, current_currency);
                    if let Some(needed_input) = calculator.calculate_offer_book_input_for_output(
                        current_amount.clone(),
                        &book_key,
                    ) {
                        current_amount = needed_input;
                        current_currency = step_currency;
                    }
                }
            }
        }

        Some(current_amount)
    }

    /// Find paths with source amount limit
    /// Returns paths that can deliver the most with the given source amount
    pub fn find_paths_with_source(
        &self,
        source: AccountID,
        destination: AccountID,
        source_amount: Amount,
        destination_currency: Currency,
    ) -> Vec<FoundPath> {
        // First find paths to destination, then filter by source amount
        // Estimate destination amount by assuming best-case rates
        let estimated_dest = Amount::issued(
            source_amount.mantissa.max(0),
            0,
            destination_currency,
            destination,
        ).unwrap_or_else(|| Amount::call(0));

        let mut paths = self.find_paths(source, destination, estimated_dest);

        // Filter and adjust paths that exceed source amount
        for path in &mut paths {
            if path.source_amount.mantissa > source_amount.mantissa.max(0) {
                // Scale down the destination amount proportionally
                let ratio = source_amount.mantissa.max(0) as f64 / path.source_amount.mantissa as f64;
                let new_dest = (path.destination_amount.mantissa as f64 * ratio) as i64;
                if let Some(new_amt) = Amount::issued(new_dest, 0, destination_currency, destination) {
                    path.destination_amount = new_amt;
                }
                path.source_amount = source_amount.clone();
            }
        }

        // Sort by destination amount (most received is best)
        paths.sort_by(|a, b| {
            b.destination_amount.mantissa.cmp(&a.destination_amount.mantissa)
        });

        paths
    }

    /// Calculate the total cost of a path (backward calculation)
    pub fn calculate_path_cost(&self, path: &[PathStep], amount: Amount) -> Option<Amount> {
        if path.is_empty() {
            return Some(amount);
        }

        // Find the source account from the first step
        let source = path.first().and_then(|s| s.account).unwrap_or_else(|| AccountID::new([0u8; 20]));
        self.calculate_source_cost(path, &amount, source)
    }

    /// Get the best path based on quality (most destination for least source)
    pub fn find_best_path(
        &self,
        source: AccountID,
        destination: AccountID,
        destination_amount: Amount,
    ) -> Option<FoundPath> {
        let paths = self.find_paths(source, destination, destination_amount);
        paths.into_iter().next() // Already sorted by quality
    }

    /// Get all paths as simple step vectors (for backward compatibility)
    pub fn find_path_steps(
        &self,
        source: AccountID,
        destination: AccountID,
        destination_amount: Amount,
    ) -> Vec<Vec<PathStep>> {
        let paths = self.find_paths(source, destination, destination_amount);
        paths.into_iter().map(|p| p.steps).collect()
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
        let _currency1 = Currency::CALL;
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

    #[test]
    fn test_trust_line_quality() {
        let account = AccountID::new([0u8; 20]);
        let issuer = AccountID::new([1u8; 20]);
        let currency = Currency::new([2u8; 20]);
        let limit = Amount::issued(1000000, 0, currency, issuer).unwrap();

        // Default quality should be 1.0
        let trust_line = TrustLine::new(account, issuer, currency, limit.clone());
        assert_eq!(trust_line.get_quality_in(), 1.0);
        assert_eq!(trust_line.get_quality_out(), 1.0);

        // Test with quality settings
        // 1000000000 = 1.0 (100% quality)
        // 2000000000 = 2.0 (200% quality - premium)
        // 500000000 = 0.5 (50% quality - discount)
        let trust_line_with_quality = TrustLine::with_quality(
            account,
            issuer,
            currency,
            limit,
            Some(2_000_000_000), // 2.0 quality in (values incoming IOUs at 2x)
            Some(500_000_000),   // 0.5 quality out (charges 2x for outgoing)
        );

        assert_eq!(trust_line_with_quality.get_quality_in(), 2.0);
        assert_eq!(trust_line_with_quality.get_quality_out(), 0.5);

        // Test apply quality in (incoming - values IOUs more)
        // With quality in of 2.0, receiving 100 IOUs is valued as 200
        let effective_in = trust_line_with_quality.apply_quality_in(100);
        assert_eq!(effective_in, 200);

        // Test apply quality out (outgoing - charges more)
        // With quality out of 0.5, to send 100 IOUs requires 200 from sender
        let required_out = trust_line_with_quality.apply_quality_out(100);
        assert_eq!(required_out, 200);
    }

    #[test]
    fn test_trust_line_quality_zero_treated_as_default() {
        let account = AccountID::new([0u8; 20]);
        let issuer = AccountID::new([1u8; 20]);
        let currency = Currency::new([2u8; 20]);
        let limit = Amount::issued(1000000, 0, currency, issuer).unwrap();

        // Quality of 0 should be treated as 1.0 (no quality set)
        let trust_line = TrustLine::with_quality(
            account,
            issuer,
            currency,
            limit,
            Some(0),
            Some(0),
        );

        assert_eq!(trust_line.get_quality_in(), 1.0);
        assert_eq!(trust_line.get_quality_out(), 1.0);
    }
}
