use sf_core::crawl::CrawlUrl;
use sf_core::filter_key::FilterKey;
use sf_core::tab::TabKey;

use crate::{EvalContext, Evaluator, Finding};

pub struct ResponseCodeEvaluator;

impl Evaluator for ResponseCodeEvaluator {
    fn tab(&self) -> TabKey {
        TabKey::ResponseCode
    }

    fn evaluate(&self, url: &CrawlUrl, _ctx: &EvalContext) -> Vec<Finding> {
        let mut findings = Vec::new();

        let code = url.status_code;
        let is_internal = url.is_internal;

        // Combined (all URLs) filters
        findings.push(Finding {
            filter_key: FilterKey::ResponseCodeAll,
        });

        match code {
            None => {
                findings.push(Finding {
                    filter_key: FilterKey::ResponseCodeNoResponse,
                });
            }
            Some(c) if c == 0 => {
                findings.push(Finding {
                    filter_key: FilterKey::ResponseCodeBlocked,
                });
            }
            Some(c) if (200..300).contains(&(c as i32)) => {
                findings.push(Finding {
                    filter_key: FilterKey::ResponseCodeSuccess,
                });
            }
            Some(c) if (300..400).contains(&(c as i32)) => {
                findings.push(Finding {
                    filter_key: FilterKey::ResponseCodeRedirection,
                });
            }
            Some(c) if (400..500).contains(&(c as i32)) => {
                findings.push(Finding {
                    filter_key: FilterKey::ResponseCodeClientError,
                });
            }
            Some(c) if (500..600).contains(&(c as i32)) => {
                findings.push(Finding {
                    filter_key: FilterKey::ResponseCodeServerError,
                });
            }
            _ => {}
        }

        // Internal/external scoped filters
        if is_internal {
            findings.push(Finding {
                filter_key: FilterKey::ResponseCodeInternalAll,
            });
            match code {
                None => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeInternalNoResponse,
                    });
                }
                Some(c) if c == 0 => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeInternalBlocked,
                    });
                }
                Some(c) if (200..300).contains(&(c as i32)) => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeInternalSuccess,
                    });
                }
                Some(c) if (300..400).contains(&(c as i32)) => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeInternalRedirection,
                    });
                }
                Some(c) if (400..500).contains(&(c as i32)) => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeInternalClientError,
                    });
                }
                Some(c) if (500..600).contains(&(c as i32)) => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeInternalServerError,
                    });
                }
                _ => {}
            }
        } else {
            findings.push(Finding {
                filter_key: FilterKey::ResponseCodeExternalAll,
            });
            match code {
                None => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeExternalNoResponse,
                    });
                }
                Some(c) if c == 0 => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeExternalBlocked,
                    });
                }
                Some(c) if (200..300).contains(&(c as i32)) => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeExternalSuccess,
                    });
                }
                Some(c) if (300..400).contains(&(c as i32)) => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeExternalRedirection,
                    });
                }
                Some(c) if (400..500).contains(&(c as i32)) => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeExternalClientError,
                    });
                }
                Some(c) if (500..600).contains(&(c as i32)) => {
                    findings.push(Finding {
                        filter_key: FilterKey::ResponseCodeExternalServerError,
                    });
                }
                _ => {}
            }
        }

        findings
    }
}
