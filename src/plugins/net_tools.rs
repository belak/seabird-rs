use crate::prelude::*;

use trust_dns_resolver::{proto::rr::RData, proto::xfer::DnsRequestOptions, AsyncResolver};

pub struct NetToolsPlugin {}

impl NetToolsPlugin {
    pub fn new() -> Self {
        NetToolsPlugin {}
    }
}

fn display_rdata(rdata: RData) -> String {
    match rdata {
        RData::A(addr) => format!("A {}", addr),
        RData::AAAA(addr) => format!("AAAA {}", addr),
        RData::ANAME(name) => format!("ANAME {}", name),
        RData::CAA(_caa) => unimplemented!(),
        RData::CNAME(name) => format!("CNAME {}", name),
        RData::MX(mx) => format!("MX {} {}", mx.preference(), mx.exchange()),
        RData::NAPTR(_naptr) => unimplemented!(),
        RData::NULL(null) => format!(
            "NULL {}",
            String::from_utf8_lossy(null.anything().unwrap_or(b"")),
        ),
        RData::NS(name) => format!("NS {}", name),
        RData::OPENPGPKEY(_key) => unimplemented!(),
        RData::OPT(_opt) => unimplemented!(),
        RData::PTR(name) => format!("PTR {}", name),
        RData::SOA(soa) => format!(
            "SOA {} {} {} {} {} {} {}",
            soa.mname(),
            soa.rname(),
            soa.serial(),
            soa.refresh(),
            soa.retry(),
            soa.expire(),
            soa.minimum()
        ),
        RData::SRV(srv) => format!(
            "SRV {} {} {} {}",
            srv.priority(),
            srv.weight(),
            srv.port(),
            srv.target()
        ),
        RData::SSHFP(_sshfp) => unimplemented!(),
        RData::TLSA(_tlsa) => unimplemented!(),
        RData::TXT(txt) => format!(
            "TXT {:?}",
            txt.txt_data()
                .iter()
                .map(|data| String::from_utf8_lossy(data))
                .collect::<Vec<_>>()
        ),
        RData::Unknown { code, rdata } => format!(
            "UNKNOWN ({}) {}",
            code,
            String::from_utf8_lossy(rdata.anything().unwrap_or(b"")),
        ),
        RData::ZERO => "ZERO".to_string(),
    }
}

#[async_trait]
impl Plugin for NetToolsPlugin {
    async fn handle_message(&self, ctx: &Arc<Context>) -> Result<()> {
        match ctx.as_event() {
            Event::Command("dig", Some(arg)) => {
                let ctx = (*ctx).clone();

                // There are some weird ownership issues if this iterator lives
                // too long, so we take care of pulling the data out as soon as
                // we can.
                let mut iter = arg.splitn(2, ' ').map(String::from);
                let arg0 = iter.next();
                let arg1 = iter.next();

                crate::spawn(async move {
                    let resolver = AsyncResolver::tokio_from_system_conf().await?;

                    let records: Vec<_> = match (arg0, arg1) {
                        // If a record_type was provided, we need to try and
                        // convert it.
                        (Some(record_type), Some(name)) => resolver
                            .lookup(name, record_type.parse()?, DnsRequestOptions::default())
                            .await?
                            .into_iter()
                            .map(display_rdata)
                            .collect(),

                        // If they didn't provide a lookup type, default to A/AAAA
                        // records.
                        (Some(name), None) => resolver
                            .lookup_ip(name)
                            .await?
                            .iter()
                            .map(|ip| ip.to_string())
                            .collect(),

                        // It should be impossible to get no results from this
                        // iterator as even the empty string will have that as the
                        // first result.
                        (None, None) => unreachable!(),
                        (None, Some(_)) => unreachable!(),
                    };

                    for record in records {
                        ctx.mention_reply(&record.to_string()).await?;
                    }

                    Ok(())
                });
            }
            Event::Command("dig", None) => ctx.mention_reply("Not enough arguments").await?,
            _ => {}
        }

        Ok(())
    }
}