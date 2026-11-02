//! Declarative subset of the OFX element hierarchy.
//!
//! The schema is shared by the SGML normalizer and, later, the SGML-to-XML
//! converter. It only describes the subset of OFX that the parser understands;
//! unmodelled elements are handled conservatively.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Whether an element is an aggregate (contains elements) or a leaf (contains text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Aggregate,
    Leaf,
}

struct AggregateSpec {
    name: &'static str,
    /// Aggregate elements that may be direct children.
    children: &'static [&'static str],
    /// Leaf elements that may be direct children.
    ///
    /// `None` means the aggregate is not fully modelled and any leaf is
    /// accepted, which keeps normalization conservative.
    leaves: Option<&'static [&'static str]>,
}

const AGGREGATES: &[AggregateSpec] = &[
    AggregateSpec {
        name: "OFX",
        children: &[
            "SIGNONMSGSRSV1",
            "SIGNONMSGSRQV1",
            "BANKMSGSRSV1",
            "BANKMSGSRQV1",
            "CREDITCARDMSGSRSV1",
            "CREDITCARDMSGSRQV1",
            "INVSTMTMSGSRSV1",
            "INVSTMTMSGSRQV1",
            "SECLISTMSGSRSV1",
        ],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "SIGNONMSGSRSV1",
        children: &["SONRS"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "SIGNONMSGSRQV1",
        children: &["SONRQ"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "SONRS",
        children: &["STATUS", "FI"],
        leaves: Some(&[
            "DTSERVER",
            "LANGUAGE",
            "DTPROFUP",
            "DTACCTUP",
            "SESSCOOKIE",
            "ACCESSKEY",
            "USERKEY",
        ]),
    },
    AggregateSpec {
        name: "SONRQ",
        children: &["STATUS", "FI"],
        leaves: Some(&[
            "DTCLIENT", "USERKEY", "USERID", "LANGUAGE", "APPID", "APPVER",
        ]),
    },
    AggregateSpec {
        name: "STATUS",
        children: &[],
        leaves: Some(&["CODE", "SEVERITY", "MESSAGE", "CODEVALIDATION", "USERKEY"]),
    },
    AggregateSpec {
        name: "FI",
        children: &[],
        leaves: Some(&["ORG", "FID"]),
    },
    AggregateSpec {
        name: "BANKMSGSRSV1",
        children: &["STMTTRNRS"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "BANKMSGSRQV1",
        children: &["STMTTRNRQ"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "STMTTRNRS",
        children: &["STATUS", "STMTRS"],
        leaves: Some(&["TRNUID", "CLTCOOKIE"]),
    },
    AggregateSpec {
        name: "STMTTRNRQ",
        children: &["STMTRQ"],
        leaves: Some(&["TRNUID", "CLTCOOKIE"]),
    },
    AggregateSpec {
        name: "STMTRS",
        children: &[
            "BANKACCTFROM",
            "BANKTRANLIST",
            "LEDGERBAL",
            "AVAILBAL",
            "BALLIST",
        ],
        leaves: Some(&["CURDEF", "MKTGINFO"]),
    },
    AggregateSpec {
        name: "STMTRQ",
        children: &["BANKACCTFROM"],
        leaves: None,
    },
    AggregateSpec {
        name: "BANKACCTFROM",
        children: &[],
        leaves: Some(&["BANKID", "BRANCHID", "ACCTID", "ACCTTYPE", "ACCTKEY"]),
    },
    AggregateSpec {
        name: "BANKACCTTO",
        children: &[],
        leaves: Some(&["BANKID", "BRANCHID", "ACCTID", "ACCTTYPE", "ACCTKEY"]),
    },
    AggregateSpec {
        name: "BANKTRANLIST",
        children: &["STMTTRN"],
        leaves: Some(&["DTSTART", "DTEND"]),
    },
    AggregateSpec {
        name: "STMTTRN",
        children: &[
            "PAYEE",
            "BANKACCTTO",
            "CCACCTTO",
            "CURRENCY",
            "ORIGCURRENCY",
        ],
        leaves: Some(&[
            "TRNTYPE",
            "DTPOSTED",
            "DTUSER",
            "DTAVAIL",
            "TRNAMT",
            "FITID",
            "SRVRTID",
            "CHECKNUM",
            "REFNUM",
            "SIC",
            "PAYEEID",
            "NAME",
            "EXTDBINFON",
            "MEMO",
            "CORRECTFITID",
            "CORRECTACTION",
            "INV401KSOURCE",
        ]),
    },
    AggregateSpec {
        name: "LEDGERBAL",
        children: &[],
        leaves: Some(&["BALAMT", "DTASOF"]),
    },
    AggregateSpec {
        name: "AVAILBAL",
        children: &[],
        leaves: Some(&["BALAMT", "DTASOF"]),
    },
    AggregateSpec {
        name: "BALLIST",
        children: &["BAL"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "BAL",
        children: &[],
        leaves: Some(&["NAME", "DESC", "BALTYPE", "VALUE", "DTASOF", "CURRENCY"]),
    },
    AggregateSpec {
        name: "PAYEE",
        children: &[],
        leaves: Some(&[
            "NAME",
            "ADDR1",
            "ADDR2",
            "ADDR3",
            "CITY",
            "STATE",
            "POSTALCODE",
            "COUNTRY",
            "PHONE",
        ]),
    },
    AggregateSpec {
        name: "CURRENCY",
        children: &[],
        leaves: Some(&["CURRATE", "CURSYM"]),
    },
    AggregateSpec {
        name: "ORIGCURRENCY",
        children: &[],
        leaves: Some(&["CURRATE", "CURSYM"]),
    },
    AggregateSpec {
        name: "CREDITCARDMSGSRSV1",
        children: &["CCSTMTTRNRS"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "CREDITCARDMSGSRQV1",
        children: &["CCSTMTTRNRQ"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "CCSTMTTRNRS",
        children: &["STATUS", "CCSTMTRS"],
        leaves: Some(&["TRNUID", "CLTCOOKIE"]),
    },
    AggregateSpec {
        name: "CCSTMTTRNRQ",
        children: &["CCSTMTRQ"],
        leaves: Some(&["TRNUID", "CLTCOOKIE"]),
    },
    AggregateSpec {
        name: "CCSTMTRS",
        children: &[
            "CCACCTFROM",
            "BANKTRANLIST",
            "LEDGERBAL",
            "AVAILBAL",
            "BALLIST",
        ],
        leaves: Some(&["CURDEF", "MKTGINFO"]),
    },
    AggregateSpec {
        name: "CCSTMTRQ",
        children: &["CCACCTFROM"],
        leaves: None,
    },
    AggregateSpec {
        name: "CCACCTFROM",
        children: &[],
        leaves: Some(&["ACCTID", "ACCTTYPE", "ACCTKEY"]),
    },
    AggregateSpec {
        name: "CCACCTTO",
        children: &[],
        leaves: Some(&["ACCTID", "ACCTTYPE", "ACCTKEY"]),
    },
    AggregateSpec {
        name: "INVSTMTMSGSRSV1",
        children: &["INVSTMTTRNRS"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "INVSTMTMSGSRQV1",
        children: &["INVSTMTTRNRQ"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "INVSTMTTRNRS",
        children: &["STATUS", "INVSTMTRS"],
        leaves: Some(&["TRNUID", "CLTCOOKIE"]),
    },
    AggregateSpec {
        name: "INVSTMTTRNRQ",
        children: &["INVSTMTRQ"],
        leaves: Some(&["TRNUID", "CLTCOOKIE"]),
    },
    AggregateSpec {
        name: "INVSTMTRS",
        children: &[
            "INVACCTFROM",
            "INVTRANLIST",
            "INVPOSLIST",
            "INVBAL",
            "INVOOLIST",
        ],
        leaves: Some(&["CURDEF", "MKTGINFO"]),
    },
    AggregateSpec {
        name: "INVSTMTRQ",
        children: &["INVACCTFROM"],
        leaves: None,
    },
    AggregateSpec {
        name: "INVACCTFROM",
        children: &[],
        leaves: Some(&["BROKERID", "ACCTID", "ACCTTYPE"]),
    },
    AggregateSpec {
        name: "INVTRANLIST",
        children: &[
            "INVBANKTRAN",
            "BUYSTOCK",
            "SELLSTOCK",
            "BUYMF",
            "SELLMF",
            "BUYOPT",
            "SELLOPT",
            "BUYOTHER",
            "SELLOTHER",
            "INCOME",
            "INVEXPENSE",
            "JRNLSEC",
            "JOURNAL",
            "TRANSFER",
            "MARGININTEREST",
            "REINVEST",
            "RETOFCAP",
            "CLOSUREOPT",
            "SPLIT",
        ],
        leaves: Some(&["DTSTART", "DTEND"]),
    },
    AggregateSpec {
        name: "INVBANKTRAN",
        children: &["STMTTRN"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "BUYSTOCK",
        children: &["INVBUY", "INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "SELLSTOCK",
        children: &["INVSELL", "INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "BUYMF",
        children: &["INVBUY", "INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "SELLMF",
        children: &["INVSELL", "INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "BUYOPT",
        children: &["INVBUY", "INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "SELLOPT",
        children: &["INVSELL", "INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "BUYOTHER",
        children: &["INVBUY", "INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "SELLOTHER",
        children: &["INVSELL", "INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "INCOME",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "INVEXPENSE",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "JRNLSEC",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "JOURNAL",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "TRANSFER",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "MARGININTEREST",
        children: &["INVTRAN", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "REINVEST",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "RETOFCAP",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "CLOSUREOPT",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "SPLIT",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "INVBUY",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "INVSELL",
        children: &["INVTRAN", "SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "INVTRAN",
        children: &[],
        leaves: None,
    },
    AggregateSpec {
        name: "INVPOSLIST",
        children: &["POSSTOCK", "POSOPT", "POSMF", "POSOTHER"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "POSSTOCK",
        children: &["INVPOS"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "POSOPT",
        children: &["INVPOS"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "POSMF",
        children: &["INVPOS"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "POSOTHER",
        children: &["INVPOS"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "INVPOS",
        children: &["SECID"],
        leaves: None,
    },
    AggregateSpec {
        name: "INVBAL",
        children: &["BALLIST"],
        leaves: None,
    },
    AggregateSpec {
        name: "INVOOLIST",
        children: &[
            "OOBUYSTOCK",
            "OOSELLSTOCK",
            "OOBUYMF",
            "OOSELLMF",
            "OOBUYOPT",
            "OOSELLOPT",
            "OOBUYOTHER",
            "OOSELLOTHER",
        ],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OOBUYSTOCK",
        children: &["OO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OOSELLSTOCK",
        children: &["OO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OOBUYMF",
        children: &["OO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OOSELLMF",
        children: &["OO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OOBUYOPT",
        children: &["OO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OOSELLOPT",
        children: &["OO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OOBUYOTHER",
        children: &["OO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OOSELLOTHER",
        children: &["OO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "OO",
        children: &["SECID", "CURRENCY", "ORIGCURRENCY"],
        leaves: None,
    },
    AggregateSpec {
        name: "SECLISTMSGSRSV1",
        children: &["SECLIST"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "SECLIST",
        children: &["STOCKINFO", "OPTINFO", "MFINFO", "DEBTINFO", "OTHERINFO"],
        leaves: Some(&[]),
    },
    AggregateSpec {
        name: "STOCKINFO",
        children: &["SECINFO", "SECID"],
        leaves: None,
    },
    AggregateSpec {
        name: "OPTINFO",
        children: &["SECINFO", "SECID"],
        leaves: None,
    },
    AggregateSpec {
        name: "MFINFO",
        children: &["SECINFO", "SECID"],
        leaves: None,
    },
    AggregateSpec {
        name: "DEBTINFO",
        children: &["SECINFO", "SECID"],
        leaves: None,
    },
    AggregateSpec {
        name: "OTHERINFO",
        children: &["SECINFO", "SECID"],
        leaves: None,
    },
    AggregateSpec {
        name: "SECINFO",
        children: &["CURRENCY", "ORIGCURRENCY"],
        leaves: Some(&[
            "SECNAME",
            "TICKER",
            "FIID",
            "RATING",
            "UNITPRICE",
            "DTASOF",
            "MEMO",
            "SECTOR",
            "ASSETCLASS",
            "DEBTCLASS",
            "COUPONRT",
            "COUPONFREQ",
        ]),
    },
    AggregateSpec {
        name: "SECID",
        children: &[],
        leaves: Some(&["UNIQUEID", "UNIQUEIDTYPE"]),
    },
    AggregateSpec {
        name: "MFACHALLENGERQ",
        children: &["CHALLENGERQ"],
        leaves: None,
    },
    AggregateSpec {
        name: "MFACHALLENGERS",
        children: &["CHALLENGERS"],
        leaves: None,
    },
    AggregateSpec {
        name: "CHALLENGERQ",
        children: &[],
        leaves: None,
    },
    AggregateSpec {
        name: "CHALLENGERS",
        children: &[],
        leaves: None,
    },
    AggregateSpec {
        name: "PINCHRQ",
        children: &[],
        leaves: None,
    },
    AggregateSpec {
        name: "PINCHRS",
        children: &[],
        leaves: None,
    },
    AggregateSpec {
        name: "OFXEXTENSION",
        children: &[],
        leaves: None,
    },
];

/// Indexed view of [`AGGREGATES`] and of all leaf names.
pub(crate) struct Schema {
    kinds: HashMap<&'static str, NodeKind>,
    aggregates: HashMap<&'static str, &'static AggregateSpec>,
}

impl Schema {
    fn build() -> Self {
        let mut kinds = HashMap::new();
        let mut aggregates = HashMap::new();

        for spec in AGGREGATES {
            kinds.insert(spec.name, NodeKind::Aggregate);
            aggregates.insert(spec.name, spec);
        }
        for spec in AGGREGATES {
            if let Some(leaves) = spec.leaves {
                for leaf in leaves {
                    kinds.entry(leaf).or_insert(NodeKind::Leaf);
                }
            }
        }

        Self { kinds, aggregates }
    }

    /// Returns the global kind of an element.
    pub(crate) fn kind(&self, name: &str) -> Option<NodeKind> {
        self.kinds.get(name).copied()
    }

    /// Returns the kind of `name` inside `parent`, taking parent-specific leaf
    /// names into account.
    ///
    /// `CURRENCY` is an aggregate under `STMTTRN` but a leaf under `BAL`; this
    /// method resolves that ambiguity.
    pub(crate) fn kind_in(&self, parent: Option<&str>, name: &str) -> Option<NodeKind> {
        if let Some(parent) = parent
            && self.accepts_leaf(parent, name) == Some(true)
        {
            return Some(NodeKind::Leaf);
        }
        self.kind(name)
    }

    /// Returns whether `parent` may directly contain the aggregate `child`.
    ///
    /// Returns `None` when `parent` is not a known aggregate.
    pub(crate) fn can_contain(&self, parent: &str, child: &str) -> Option<bool> {
        self.aggregates
            .get(parent)
            .map(|spec| spec.children.contains(&child))
    }

    /// Returns whether `parent` may directly contain the leaf `leaf`.
    ///
    /// Returns `None` when `parent` is not a known aggregate. Aggregates with
    /// unmodelled leaves accept any leaf.
    pub(crate) fn accepts_leaf(&self, parent: &str, leaf: &str) -> Option<bool> {
        self.aggregates.get(parent).map(|spec| match spec.leaves {
            Some(leaves) => leaves.contains(&leaf),
            None => true,
        })
    }
}

static SCHEMA: LazyLock<Schema> = LazyLock::new(Schema::build);

/// Returns the process-wide OFX schema.
pub(crate) fn schema() -> &'static Schema {
    &SCHEMA
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_child_is_defined() {
        let schema = schema();
        for spec in AGGREGATES {
            for child in spec.children {
                assert!(
                    schema.kind(child).is_some(),
                    "child {child} of {} is not defined",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn every_leaf_parent_is_an_aggregate() {
        let schema = schema();
        for name in schema.kinds.keys() {
            assert!(schema.kinds.contains_key(name), "missing kind for {name}");
        }
        for spec in AGGREGATES {
            assert_eq!(schema.kind(spec.name), Some(NodeKind::Aggregate));
        }
    }

    #[test]
    fn currency_is_a_leaf_under_bal() {
        let schema = schema();
        assert_eq!(schema.kind("CURRENCY"), Some(NodeKind::Aggregate));
        assert_eq!(
            schema.kind_in(Some("BAL"), "CURRENCY"),
            Some(NodeKind::Leaf)
        );
        assert_eq!(
            schema.kind_in(Some("STMTTRN"), "CURRENCY"),
            Some(NodeKind::Aggregate)
        );
    }

    #[test]
    fn known_relationships_are_available() {
        let schema = schema();
        assert_eq!(schema.can_contain("STMTTRNRS", "STMTRS"), Some(true));
        assert_eq!(schema.can_contain("BANKACCTFROM", "ACCTKEY"), Some(false));
        assert_eq!(schema.accepts_leaf("BANKACCTFROM", "ACCTKEY"), Some(true));
        assert_eq!(schema.accepts_leaf("STATUS", "BOGUS"), Some(false));
    }
}
