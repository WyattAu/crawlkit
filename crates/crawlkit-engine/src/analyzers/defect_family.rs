//! Curated defect families: codes that describe **one** defect.
//!
//! # Why this is a hand-maintained table
//!
//! Analyzer families were grown additively and many groups ended up checking the
//! same property with different wording. Comparing titles cannot fix this,
//! because the wordings differ:
//!
//! ```text
//! A11Y006  "Missing main landmark"
//! LAND001  "Missing main landmark region"
//! LANDMAIN001 "Page missing main landmark"
//! ```
//!
//! Approximating "close enough" by token similarity is **unsafe**. Measured over
//! the corpus, a 50%-Jaccard title match proposed 46 clusters, and several were
//! plainly different defects that would have been silently merged:
//!
//! ```text
//! "Location entities detected"   vs "Organization entities detected"
//! "Description too long"         vs "Title too long"
//! "Fairly difficult readability" vs "Very difficult readability"
//! "External scripts missing integrity attribute" vs "External stylesheets missing ..."
//! "Description missing title keywords" vs "Title missing description keywords"
//! ```
//!
//! Merging any of those would destroy real signal. So the groupings are declared
//! explicitly and reviewed by hand. A code not listed here falls back to the
//! normalized-title signature in [`crate::analyzers::dedupe`], which is exact
//! after decoration stripping and therefore cannot merge distinct defects.
//!
//! # Invariant
//!
//! Two codes may share a family **only** if every one of them reports the same
//! property on the same object. Differing thresholds (thin vs *extremely* thin),
//! differing subjects (scripts vs stylesheets), or differing directions
//! (missing title keywords vs missing description keywords) are different
//! defects and must stay separate.

/// Groups of codes that all report one defect.
///
/// Ordered by descending family size. Ids are stable strings so they can be
/// logged and asserted in tests.
/// Canonical defect title for each family of interchangeable codes.
///
/// The key is the title the defect is canonically reported under; the value is
/// every code that reports it. Deduplication resolves a finding's key by
/// looking its code up here and falling back to the finding's own normalized
/// title, then compares keys — so both exact-title siblings and these reviewed
/// synonym families converge without either mechanism being able to disagree.
pub const CANONICAL_TITLES: &[(&str, &[&str])] = &[
    // Cross-page duplicate-title reporting. `CrossPageDuplicateContentDetector`
    // and `KeywordCannibalizationAnalyzer` both report duplicate titles under
    // their own codes, so every set of title collisions produced two rows per
    // page. They reached `dedupe.rs` only once post-crawl registry findings
    // became persistable.
    //
    // `DUP-CROSS002` (duplicate descriptions) and `CANNIB001` (cannibalization)
    // stay separate: they describe different defects and collapsing them would
    // hide the distinction between "the same description twice" and "these pages
    // compete for one query".
    ("duplicate titles across pages", &["DUP-CROSS001", "KEY-CANNIB001"]),
    ("meta description too short", &["MDESC-PX002", "META005", "METADEEP-V2001", "METADEEP001", "METADESC001", "METADESSSHORT-V2001", "METADESSSHORT-V6086", "METADESSSHORT001", "METALEN-V2002", "METAQLT-V2001"]),
    ("missing hreflang x-default", &["HREF-V3001", "HREF-V4001", "HREF001", "HREFNET002", "HREFRECIP001", "HREFXD-V2001", "HREFXD-V5001", "HREFXD-V6094", "HREFXD001", "ISEO002"]),
    ("multiple h1 headings", &["A11Y004", "CDEPTH003", "H1MULTI-V6108", "HEAD003", "HEADH1-V5002", "HEADSC003", "HHIER-V2003-DEEP-DEEP", "HHIER-V2003-DEEP-DEEP-DEEP", "HHIERDEEP003", "HEADING-MULTIH1"]),
    ("tables missing captions", &["A11Y015", "TABACC-V2002", "TABACC-V2002-DEEP-DEEP-DEEP", "TABACCDEEP002", "TABCAP-V6112", "TABLECAP001", "TACC002", "TBLCAP001", "TBLCAPT-V5001"]),
    ("missing h1 heading", &["A11Y003", "CDEPTH002", "H1COUNT-V6107", "HEAD002", "HEADH1-V5001", "HEADSC002", "HHIER-V2002-DEEP-DEEP", "HHIER-V2002-DEEP-DEEP-DEEP"]),
    ("missing x-content-type-options header", &["CT-V2001", "CTSNIFF001", "SEC005", "XCTO-V2001", "XCTO-V2001-DEEP", "XCTO-V5001", "XCTO001", "XCTODEEP001"]),
    ("missing cross-origin-embedder-policy", &["COEP-V2001", "COEP-V5001", "COEP001-ISOLATION", "COISO-V2001", "COISO-V2001-DEEP-DEEP", "COISODEEP001", "SEC009"]),
    ("missing cross-origin-opener-policy", &["COISO-V2002", "COISO-V2003", "COISODEEP002", "COOP-V2001", "COOP-V5001", "COOP002-ISOLATION", "SEC010"]),
    ("missing main landmark", &["A11Y006", "ARIALAND-V2002", "ARIALAND001", "LAND001", "LANDMAIN-V2001", "LANDMAIN-V5001", "LANDMAIN001"]),
    ("missing navigation landmark", &["A11Y007", "ARIALAND-V2003", "ARIALAND002", "LAND002", "LANDNAV-V2001", "LANDNAV-V5001", "LANDNAV001"]),
    ("title too short", &["META002", "TITLE-V4002", "TITLE001", "TITLEDEEP001", "TITLELEN-V2002", "TITLELEN-V5002", "TITLEQLT-V2001"]),
    ("hsts missing includesubdomains", &["HSTS-V2003", "HSTS-V3001", "HSTS001", "HSTSPR-V2001", "HSTSSUB-V5001", "SEC015"]),
    ("missing canonical url", &["AI-CIT001", "CAN-V3001", "CANMISS-V2001", "CANMISS-V6088", "CANMISS001", "CANON001"]),
    ("missing referrer-policy header", &["REF-V2001", "REF001", "RP-V5001", "RPDEEP-V2001", "RPDEEP001", "SEC007"]),
    ("canonical does not self-reference", &["CANCHAIN-V5001", "CANSELF-V5001", "CANSELFRF-V2001", "CANSELFRF-V6089", "CANSELFRF001"]),
    ("meta description too long", &["META-V3001-LENGTH", "META006", "METADEEP002", "METADESC002", "METALEN-V2001"]),
    ("missing html lang attribute", &["A11Y016", "LANG001", "LANGACC001", "LANGATTR-V2001", "LANGATTRDEEP001"]),
    ("missing permissions-policy header", &["PERM001", "PERMP-V2001", "PERMP-V2001-DEEP-DEEP", "PERMPDEEP001", "SEC008"]),
    ("no headings found", &["A11Y002", "HEAD001", "HEADSC001", "HHIER-V2001", "HHIER-V2001-DEEP-DEEP-DEEP", "HEADING-NONE"]),
    ("tables without headers", &["TABACC-V2001", "TABACC-V2001-DEEP-DEEP", "TABACC-V2001-DEEP-DEEP-DEEP", "TABACCDEEP001", "TBL-V2001"]),
    ("webapi missing documentation", &["WAPI-V2001", "WAPI002", "WAPIDOC-V2001", "WAPIDOC-V6049", "WAPIDOCS001"]),
    ("webpageelement missing name", &["WELEM-V2001", "WELEM001", "WPELACC001", "WPELNAME-V2001", "WPELNAME-V6051"]),
    ("article missing author", &["ART-AUTH001", "ART003", "ARTAUTH-V2001", "ARTQUAL002"]),
    ("article missing datepublished", &["ART-DT001", "ART002", "ARTDT-V2001", "ARTQUAL003"]),
    ("article missing headline", &["ART-HL001", "ART001", "ARTHL-V2001", "ARTQUAL001"]),
    ("car missing model", &["CAR-V2001", "CAR002", "CARMODEL-V2001", "CARMODEL001"]),
    ("event missing location", &["ELOC-V2001", "ELOC001", "EVENT002", "EVTLOC-V6059"]),
    ("heading level skipped", &["A11Y005", "HCOV002", "HEAD004", "HEADSC004", "HEADSKIP001", "HEADSKIP-V2001", "HEADSKIP-V5001", "HEADSKIP-V6121", "HHIER-V2003", "HHIER-V2004", "HORDER001", "HEADING-SKIPLEVEL"]),
    ("healthplan missing provider", &["HP-V2001", "HP002", "HPPRV-V2001", "HPPRV-V6036"]),
    ("images missing alt attribute", &["A11Y001", "IMG-V2001", "IMGALT-V2001-DEEP-DEEP", "IMGALT-V2001-DEEP-DEEP-DEEP"]),
    ("missing banner landmark", &["LAND003", "LANDBAN-V2001", "LANDBAN001", "LANDBANNER-V5001"]),
    ("movie missing director", &["MOVDIR-V2001", "MOVDIR-V6027", "MOVIE-V2001", "MOVIE002"]),
    ("permit missing permitnumber", &["PERM-V2001-SCHEMA", "PERMIT001", "PERMNUM-V2001", "PERMNUM-V6040"]),
    ("plan missing description", &["PLAN-V2001", "PLAN002", "PLANDESC-V2001", "PLANDESC-V6042"]),
    ("researchproject missing about", &["RPABOUT-V2001", "RPABOUT-V6044", "RPROJ-V2001", "RPROJ002"]),
    ("title too long", &["META003", "TITLE002", "TITLEDEEP002", "TITLELEN-V2001"]),
    ("trip missing itinerary", &["TRIP-V2001", "TRIP002", "TRIPIT-V2001", "TRIPIT-V6047"]),
    ("worker missing jobtitle", &["WORKER-V2001", "WORKER002", "WORKJOB-V2001", "WORKJOB-V6052"]),
    ("workersunion missing name", &["WUNAME-V2001", "WUNAME-V6048", "WUNION-V2001", "WUNION001"]),
    ("apartment missing numberofrooms", &["APT-V2001", "APT002", "APTROOM001"]),
    ("aria roles without labels", &["A11Y013", "ARIALABEL001", "ARIAREQ-V2001"]),
    ("blocking scripts detected", &["CRIT001", "SCRIPT002", "SCRIPTBLK-V5001"]),
    ("book missing isbn", &["BOOK001", "BOOKISBN-V2001", "BOOKISBN-V6025"]),
    ("course missing name", &["COURSE-NAME001", "COURSE001", "COURSENAME-V2001"]),
    ("csp missing form-action", &["CSPDIR-V2002", "CSPFORM-V2001", "CSPFORM-V6080"]),
    ("csp script-src allows unsafe-inline", &["CSP001", "CSPSS-V2002", "CSPSSRC-V5001"]),
    ("duplicate hreflang language", &["HREF003", "HREFNET001", "ISEO003"]),
    ("empty link text", &["A11Y009", "LINKEEMPTY-V6114", "LINKTQ-V2002-V2"]),
    ("external scripts missing integrity attribute", &["SRI-V5001", "SRI001", "SRISCRIPT001"]),
    ("form inputs without labels", &["FORMLBL-V2001-DEEP-DEEP", "FORMLBL-V2001-DEEP-DEEP-DEEP", "FORMLBLASSOC-V5001"]),
    ("generic link text", &["LINKGEN-V6113", "LINKTQ-V2001", "LINKTQ-V2002-DEEP"]),
    ("howto missing name", &["HOWNAME001", "HOWTO001", "HOWTONAME-V5001"]),
    ("hsts missing preload", &["HSTSPR-V2002", "HSTSPRE-V5001", "SEC016"]),
    ("hsts preload readiness incomplete", &["HSTSPR-V2001-DEEP-DEEP", "HSTSPR001-DEEP", "HSTSPRELIST-V6064"]),
    ("images missing alt text", &["IMGALT-V2001", "IMGALTDEEP001", "IMGALTMISS-V6116"]),
    ("images missing dimensions", &["IMG004", "IMGDIM-V5001", "IMGDIM001"]),
    ("invalid x-content-type-options value", &["CT-V2002", "SEC006", "XCTODEEP002"]),
    ("invalid x-frame-options value", &["SEC004", "XFO-V5002", "XFODEEP002"]),
    ("invoice missing account", &["INV-V2001", "INVACCT-V2001", "INVACCT-V6038"]),
    ("jobposting missing title", &["JOB-TITLE001", "JOB001", "JOBTITLE-V2001"]),
    ("low internal link diversity", &["INTDIV-V2001", "INTDIV-V6103", "INTDIV001"]),
    ("missing contentinfo landmark", &["ARIALAND-V2004", "LANDCINFO-V2001", "LANDCINFO-V6124"]),
    ("missing meta description", &["META-V3001", "META004", "METADESCMISS-V6085"]),
    ("missing opensearch description", &["OPDESC-V2001", "OPDESC001", "OPSEARCH001"]),
    ("missing strict-transport-security header", &["HSTS-V2001", "SEC002", "STRICT001"]),
    ("missing title tag", &["TITLE-V4001", "TITLELEN-V5001", "TITLEMISS-V6081"]),
    ("missing x-frame-options header", &["SEC003", "XFO-V2001", "XFO001"]),
    ("multiple canonical tags", &["CANCHAIN-V2001", "CANCHAIN-V6090", "CANCHAIN001"]),
    ("no clickjacking protection", &["XFO-V5001", "XFODEEP001", "XFOMISS-V6072"]),
    ("no hreflang tags", &["HREFMISS-V2001", "HREFMISS-V6092", "HREFMISS001"]),
    ("no robots.txt", &["AI-ACC009", "ROBOTS-V3001", "SITEMAP-V3001"]),
    ("no sitemap in robots.txt", &["SITEMAPCOV-V5001", "SITEMAPDEEP-V2001", "SITEMAPMISS-V6096"]),
    ("organization missing name", &["ORG-NAME001", "ORG001", "ORGNAME-V2001"]),
    ("person missing name", &["PERS-NAME001", "PERS001", "PERSNAME-V2001"]),
    ("positive tabindex found", &["FOCUS-V2001", "FOCUS-V2001-DEEP-DEEP", "FOCUS-V2001-DEEP-DEEP-DEEP"]),
    ("recipe missing cooktime", &["RECCOOK001", "RECIPE002", "RECIPECOOK001"]),
    ("recipe missing name", &["RECIPE-NAME001", "RECIPE001", "RECIPENAME-V2001"]),
    ("robots.txt blocks all crawlers", &["ROBOTSDEEP001", "ROBOTSDIS-V5001", "ROBOTSWILD-V6101"]),
    ("schedule missing scheduletimezone", &["SCHED-V2001", "SCHED002", "SCHEDTZ-V6046"]),
    ("service missing provider", &["SVC002", "SVCPRV-V2001", "SVCPRV-V6035"]),
    ("title may truncate", &["TITLE-V4003", "TITLELEN-V5003", "TITLEQLT-V2002"]),
    ("tvseries missing numberofepisodes", &["TV-V2001", "TV002", "TVEPISODES001"]),
    ("wearable missing devicetype", &["WEAR002", "WEARDEV-V2001", "WEARDEV-V6050"]),
    ("action missing actiontype", &["ACTION-V2001", "ACTION001"]),
    ("aria roles without accessible names", &["ARIA001", "ARIAROLE001"]),
    ("book missing author", &["BOOKAUTH-V2001", "BOOKAUTH-V6024"]),
    ("book missing datepublished", &["BOOKDATE-V2001", "BOOKDATE-V6026"]),
    ("broadcastevent missing name", &["BENAME-V6020", "BROADCAST001"]),
    ("canonical url mismatch", &["CAN-V2001", "CANON003"]),
    ("canonical url points to different domain", &["CANCH001", "CANCON003"]),
    ("car missing manufacturer", &["CAR003", "CARMFR001"]),
    ("civicstructure missing name", &["CIVIC001", "CVNAME-V6010"]),
    ("cookie missing httponly flag", &["COOKIE002", "COOKIEHTTP001"]),
    ("cookie missing secure flag", &["COOKIE001", "COOKIESEC001-VALIDATOR"]),
    ("cors allows all origins", &["CORS002", "CORSWILD-V6069"]),
    ("cors wildcard with credentials", &["CORS-V2001", "CORS001"]),
    ("course missing provider", &["COURSE002", "CPROV-V2001"]),
    ("course provider missing name", &["COURSEPV-V5001", "CPROV001"]),
    ("creativework missing name", &["CREATIVE001", "CWNAME-V6001"]),
    ("csp connect-src allows wildcard", &["CSPCON-V2001", "CSPCON001"]),
    ("csp font-src allows wildcard", &["CSPFNT-V2001", "CSPFNT001"]),
    ("csp missing base-uri", &["CSPBASE-V6079", "CSPDIR-V2001"]),
    ("csp missing frame-ancestors", &["CSPDIR-V2003", "CSPFRAME-V5001"]),
    ("csp script-src allows unsafe-eval", &["CSPSS-V2003", "CSPSSRC-V5002"]),
    ("csp style-src allows unsafe-inline", &["CSPSTY-V2002", "CSPSTYLE-V5001"]),
    ("description may truncate", &["META-V3003", "METAQLT-V2002"]),
    ("educationalorganization missing name", &["EDUNAME-V6017", "EDUORG001"]),
    ("empty canonical url", &["CAN-V3002", "CANCON001"]),
    ("empty robots.txt", &["ROBOTSEMPTY-V2001", "ROBOTSEMPTY-V6099"]),
    ("event location missing name", &["ELOC-V2002", "ELOC002"]),
    ("event missing organizer", &["EVENT003", "EVTORG-V5002"]),
    ("event missing startdate", &["EVENT001", "EVTSTART-V6058"]),
    ("few external links use nofollow", &["EXTNOFOLLOW-V6106", "NOFOLLOW002"]),
    ("foodestablishment missing servescuisine", &["FECUIS-V6006", "FOOD003"]),
    ("form input missing associated label", &["FILABEL001", "FLABEL001"]),
    ("generic anchor text", &["ANCHGEN-V2001", "ANCHGEN-V2001-DEEP"]),
    ("governmentservice missing provider", &["GOV-V2001", "GOV002"]),
    ("high crawl-delay value", &["ROBOT003", "ROBOTSDEEP002"]),
    ("hsts max-age below 1 year", &["HSTSMAX-V2001", "HSTSMAX001"]),
    ("hsts max-age is too short", &["HSTS-V2002", "STRICT002"]),
    ("hsts missing preload directive", &["HSTS002", "HSTSPR002"]),
    ("http resources on https page", &["MIXCONT-V2001", "MIXED001"]),
    ("http script sources on https page", &["MIXSCR001", "MIXSCRIPT-V5001"]),
    ("image missing alt text", &["IMG001", "IMGALT001"]),
    ("internal links with empty anchor text", &["A11Y-LINK-V2001", "INTLINKQ003"]),
    ("invalid hreflang locale code", &["HREF002", "HREFT003"]),
    ("invalid x-content-type-options", &["XCTO-V2002", "XCTO-V5002"]),
    ("jobposting missing validthrough", &["JOB003", "JOBVT-V5001"]),
    ("json-ld type is missing", &["JLDTYPE001", "JSONLD003"]),
    ("landform missing name", &["LANDFORM001", "LFNAME-V6011"]),
    ("link with generic text", &["LINKTEXT002", "LNKACC002"]),
    ("links without text", &["LINKTQ-V2001-DEEP", "LINKTQ-V2001-DEEP-DEEP"]),
    ("localbusiness missing address", &["LBIZ002", "NAP-UTIL002"]),
    ("localbusiness missing name", &["LBIZ001", "NAP-UTIL003"]),
    ("localbusiness missing openinghours", &["LBH001", "NAP002"]),
    ("localbusiness missing telephone", &["NAP-UTIL001", "NAP001"]),
    ("long average sentence length", &["CREAD003", "WC004"]),
    ("low authority link ratio", &["EXTAUTHDP-V2001", "EXTAUTHDP003"]),
    ("low title-description keyword overlap", &["TITLEKDEN-V2001", "TITLEKDEN001"]),
    ("many deeply nested internal links", &["INTDEPTH-V2001", "INTDEPTH001"]),
    ("meta description identical to title", &["METADESC003", "METADESCUNIQ-V2001"]),
    ("missing cross-origin-resource-policy header", &["CORP001", "SEC011"]),
    ("missing date metadata", &["AI-CS008", "FRESHSC001"]),
    ("missing lang attribute", &["LANG-V2001", "LANGATTR-V2001-DEEP-DEEP"]),
    ("missing preconnect for external origins", &["CONN002", "CRIT003"]),
    ("missing self-referencing hreflang", &["HREFRECIP-V5001", "HREFSELF001"]),
    ("missing skip navigation link", &["A11Y008", "SKIPLINK001"]),
    ("missing twitter card type", &["SOCIAL004", "TW001"]),
    ("missing viewport meta tag", &["META009", "MOB001"]),
    ("missing wasm module preload", &["WASM-P004", "WASM001"]),
    ("mixed content in iframes", &["MIXIFRAME-V2001", "MIXIFRAME001"]),
    ("movie missing datecreated", &["MOVDATE-V6029", "MOVIE003"]),
    ("musicalbum missing byartist", &["MUSALB-V2001", "MUSALB002"]),
    ("ngo missing name", &["NGO001", "NGONAME-V6018"]),
    ("no aria landmarks found", &["ARIALAND-V2001", "ARIALAND-V2006"]),
    ("no complementary landmark", &["LANDCOMP-V2001", "LANDCOMP-V6125"]),
    ("no data found", &["AI-CIT005", "SD001"]),
    ("no date metadata on time-sensitive content", &["FRESH-DATE001", "FRESH001"]),
    ("no links on page", &["LINKSC001", "PIMP001"]),
    ("no visible focus indicators found", &["A11Y-FOCUS002", "FOCUS002"]),
    ("non-standard context", &["SCHEMACOV002", "SD003"]),
    ("occupation missing occupationalcategory", &["OCCUP-V2001", "OCCUP002"]),
    ("organization missing logo", &["OLOGO001", "ORG003"]),
    ("organization missing url", &["ORG002", "ORGURL-V5001"]),
    ("orphan page", &["LINK006", "ORPHAN001"]),
    ("page not found in sitemap", &["SITEMAP-COV001", "SITEMAP002"]),
    ("payment api access not explicitly denied", &["PPAYDP001", "PPPAY-V6073"]),
    ("performingartsseries missing name", &["PASNAME-V6019", "PERFORMARTS001"]),
    ("permissions-policy header missing", &["PERM-V2001", "PPERM001"]),
    ("permissions-policy missing camera restriction", &["PERM-V3001", "PERMPDEEP002"]),
    ("permit missing issuedby", &["PERMISS-V6041", "PERMIT002"]),
    ("person missing jobtitle", &["PJOB-V2001", "PJOB001"]),
    ("playbook missing name", &["PLAYBOOK-V2001", "PLAYBOOK001"]),
    ("positive tabindex detected", &["FOCUSDEEP001", "FOCUSTABPOS-V6119"]),
    ("positive tabindex values detected", &["A11Y012", "TABINDEX001"]),
    ("positive tabindex values disrupt focus order", &["FOCUS001", "TABPOS001"]),
    ("productmodel missing brand", &["PMODEL-V2001", "PMODEL002"]),
    ("recipe missing nutrition information", &["RNUT-V2001", "RNUT001"]),
    ("recipe missing preptime", &["RECIPECOOK002", "RECIPEPT-V5001"]),
    ("redirect loop detected", &["REDIR002", "REDIRLOOP001"]),
    ("referrer-policy not using strict policy", &["RPSTRICT-V2001", "RPSTRICT001"]),
    ("referrer-policy set to unsafe-url", &["REF002", "RPDEEP002"]),
    ("referrer-policy unsafe-url", &["RP-V5002", "RPDEEP-V2002"]),
    ("render-blocking scripts without async/defer", &["ASYNC001", "SCRIPT-V2001"]),
    ("required fields missing labels", &["FORMREQ-V5001", "FORMREQ-V6110"]),
    ("robots.txt blocks all", &["ROBOTS-V3002", "ROBOTSDEEP-V2001"]),
    ("service missing areaserved", &["SVC-V2001", "SVCAREA-V6034"]),
    ("sportsevent missing name", &["SENAME-V6016", "SPORTSEVT001"]),
    ("table headers missing scope", &["TBLSCOP-V2001", "TBLSCOPE-V5001"]),
    ("table missing header cells", &["A11Y014", "TACC001"]),
    ("thin content", &["CQ004", "THIN001"]),
    ("touristattraction missing name", &["TANAME-V6013", "TOURIST001"]),
    ("touristdestination missing name", &["TDNAME-V6014", "TOURISTDEST001"]),
    ("videoobject missing duration", &["VID003", "VIDDUR001"]),
    ("videoobject missing embedurl", &["VID001", "VIDEMB001"]),
    ("web fonts missing font-display:swap", &["FONT-V2001", "FONT001"]),
    ("website missing url", &["WSITE-V2001", "WSITE002"]),
    ("x-content-type-options not set to nosniff", &["CTSNIFF002", "XCTO002"]),
];

/// Title a code's defect is canonically reported under.
///
/// Every code maps to a canonical *title*, and deduplication keys on that alone.
///
/// Doing it in title space rather than "family id if the code is listed,
/// otherwise its own title" matters because of codes with dynamic titles.
/// `SD006` builds its title at runtime (`"Article missing headline"`), so the
/// static scan cannot place it in a family; under the id-based scheme it fell
/// back to a title key while the static sibling `ART-HL001` resolved to a
/// family id, and the two stopped merging — which doubled gov.uk's error tier.
///
/// Canonicalizing instead means a family and a dynamically-titled sibling
/// converge on the same string, so the two schemes cannot disagree.
fn canonical_table() -> &'static std::collections::HashMap<&'static str, &'static str> {
    use std::sync::OnceLock;
    static TABLE: OnceLock<std::collections::HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| {
        // Family members are canonicalized onto the family's most representative
        // title. `CANONICAL_TITLES` records the literal each family speaks in.
        let mut map = std::collections::HashMap::new();
        for (title, codes) in CANONICAL_TITLES {
            for code in *codes {
                assert!(
                    !map.contains_key(code),
                    "code {code} is listed under two canonical titles"
                );
                map.insert(*code, *title);
            }
        }
        map
    })
}

/// Stable identity for a finding's defect.
///
/// Unlisted codes key on their own normalized title, so exact-title siblings
/// merge automatically and no maintenance is needed for them.
#[must_use]
pub fn defect_key(code: &str, normalized_title: &str) -> String {
    match canonical_table().get(code) {
        Some(canonical) => (*canonical).to_string(),
        None => normalized_title.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::dedupe::defect_signature;

    #[test]
    fn no_code_appears_in_two_families() {
        // Exercised by the assertion inside `canonical_table()`; calling it triggers that.
        assert!(!canonical_table().is_empty());
    }

    #[test]
    fn family_members_share_one_key() {
        let title = "missing main landmark";
        let a = defect_key("A11Y006", title);
        let b = defect_key("LAND001", title);
        let c = defect_key("LANDMAIN001", title);
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn title_is_irrelevant_within_a_family() {
        // Different wordings, same reviewed defect.
        assert_eq!(
            defect_key("A11Y006", "missing main landmark"),
            defect_key("LANDMAIN001", "page missing main landmark region")
        );
    }

    #[test]
    fn unlisted_codes_fall_back_to_their_title() {
        assert_eq!(
            defect_key("SOMENEW001", "title too long"),
            defect_key("OTHER9", "title too long")
        );
        assert_ne!(
            defect_key("SOMENEW001", "title too long"),
            defect_key("SOMENEW001", "description too long")
        );
    }

    /// The pairs that a fuzzy matcher wrongly proposed as duplicates. Each must
    /// resolve to a *different* key.
    #[test]
    fn families_collapse_through_the_real_registry() {
        // End-to-end: a page with no landmarks at all must report each missing
        // landmark once, not once per contributing code.
        use crate::analyzers::AnalyzerRegistry;
        use crate::parser::HtmlParser;
        use std::time::Duration;

        let html = "<html><head><title>t</title></head><body><p>x</p></body></html>";
        let url = url::Url::parse("https://example.com/").expect("valid url");
        let page = HtmlParser::parse(html, &url);
        let headers: Vec<(String, String)> = Vec::new();
        let redirects: Vec<crate::RedirectHop> = Vec::new();
        let ctx = crate::analyzers::AnalysisContext {
            page: &page,
            body: Some(html),
            status_code: Some(200),
            headers: &headers,
            response_time: Some(Duration::from_millis(1)),
            redirect_chain: &redirects,
            robots_txt: None,
            user_agent: None,
            body_size: Some(html.len()),
            compressed_size: None,
            content_encoding: None,
            server: None,
            content_type: Some("text/html"),
            rendered: None,
        };

        let registry = AnalyzerRegistry::new(&crate::CrawlConfig::default());
        let findings = registry.analyze(&ctx);

        let count_main = findings.iter().filter(|f| f.code == "A11Y006").count()
            + findings.iter().filter(|f| f.code == "LAND001").count()
            + findings.iter().filter(|f| f.code == "LANDMAIN001").count();
        assert_eq!(
            count_main, 1,
            "the missing-main-landmark family must report once, got {count_main}"
        );

        let count_nav = findings.iter().filter(|f| f.code == "A11Y007").count()
            + findings.iter().filter(|f| f.code == "LAND002").count()
            + findings.iter().filter(|f| f.code == "LANDNAV001").count();
        assert_eq!(count_nav, 1, "the navigation-landmark family must report once");

        // The collapse must be disclosed, not silent.
        let main = findings
            .iter()
            .find(|f| {
                matches!(f.code.as_str(), "A11Y006" | "LAND001" | "LANDMAIN001")
            })
            .expect("a main-landmark finding survives");
        assert!(
            main.description.contains("also reported by"),
            "collapsed finding must list its aliases: {main:?}"
        );
    }

    #[test]
    fn genuinely_distinct_defects_are_never_merged() {
        // Entity kinds.
        assert_ne!(
            defect_key("ENTITY003", "location entities detected"),
            defect_key("ENTITY002", "organization entities detected")
        );
        // Subject of the length complaint.
        assert_ne!(
            defect_key("META003", "title too long"),
            defect_key("METADEEP002", "meta description too long")
        );
        // Readability bands.
        assert_ne!(
            defect_key("CREAD001", "very difficult readability"),
            defect_key("CREAD002", "fairly difficult readability")
        );
        // Scripts vs stylesheets.
        assert_ne!(
            defect_key("SRI001", "external scripts missing integrity attribute"),
            defect_key("SRI002", "external stylesheets missing integrity attribute")
        );
        // Direction of the keyword comparison.
        assert_ne!(
            defect_key("METAKEY-V5001", "description missing title keywords"),
            defect_key("TITLEKWP-V5001", "title missing description keywords")
        );
        // RDFa: two distinct missing attributes on the same element.
        assert_ne!(
            defect_key("RDFA001", "rdfa attributes present but missing vocab"),
            defect_key("RDFA002", "rdfa attributes present but missing typeof")
        );
        // Thin vs extremely thin are different thresholds, so different defects.
        assert_ne!(
            defect_key("CQ004", "thin content"),
            defect_key("THIN002", "extremely thin content")
        );
    }

    /// The pairs the audit found *similar* and deliberately rejected. If a future
    /// edit merges any of these, it has destroyed a real distinction.
    #[test]
    fn audited_similar_but_distinct_pairs_stay_separate() {
        // Different CSP directives.
        assert_ne!(
            defect_key("CSP001", "csp script src allows unsafe inline"),
            defect_key("CSPDIR002", "csp style src allows unsafe inline")
        );
        // Opposite directions of the same comparison.
        assert_ne!(
            defect_key(
                "METAKEY-V5001",
                "description missing title keywords"
            ),
            defect_key(
                "TITLEKWP-V5001",
                "title missing description keywords"
            )
        );
        // Different thresholds on the same measurement.
        assert_ne!(
            defect_key("CQ004", "thin content"),
            defect_key("THIN002", "extremely thin content")
        );
        // Preload eligibility is a separate, distinct remediation.
        assert_ne!(
            defect_key("HSTS-V3001", "hsts missing includesubdomains"),
            defect_key(
                "HSTSPR001",
                "hsts missing includesubdomains for preload"
            )
        );
        // Scoped vs generic lazy-loading check.
        assert_ne!(
            defect_key(
                "LAZYIMG002",
                "above the fold images with lazy loading"
            ),
            defect_key("IMG-LAZY001", "images without lazy loading")
        );
        // Different inputs: heading terms vs TF-IDF over the body.
        assert_ne!(
            defect_key("CQ002", "top keywords"),
            defect_key("KW001", "top tf idf keywords")
        );
    }

    /// Each accepted family from the live-site audit really does collapse.
    ///
    /// Asserts members share a key, not a particular key *value* — family ids
    /// are positional and are expected to change as the table is regenerated.
    #[test]
    fn audited_accepted_families_collapse() {
        let cases: &[(&[&str], &str)] = &[
            (&["CSPDIR-V2002", "CSPFORM-V2001", "CSPFORM-V6080"], "csp missing form action"),
            (
                &["METADEEP-V2001", "META005", "METADEEP001", "MDESC-PX002"],
                "meta description too short",
            ),
            (&["META003", "TITLE002", "TITLELEN-V2001"], "title too long"),
            (
                &["HSTS-V3001", "HSTS001", "HSTS-V2003", "SEC015"],
                "hsts missing includesubdomains",
            ),
            (
                &["HREFMISS-V2001", "HREFMISS-V6092", "HREFMISS001"],
                "missing hreflang tags",
            ),
            (
                &["PERMPDEEP002", "PERM-V3001"],
                "permissions policy missing camera",
            ),
            (&["CQ004", "THIN001"], "thin content"),
        ];
        for (codes, title) in cases {
            let keys: Vec<String> = codes.iter().map(|c| defect_key(c, title)).collect();
            for k in &keys {
                assert_eq!(
                    *k, keys[0],
                    "{codes:?} should all resolve to one key, got {k}"
                );
            }
        }
    }

    /// Regression: a code with a *runtime-built* title must still merge with its
    /// statically-listed siblings.
    ///
    /// `SD006` builds `"Article missing headline"` at runtime, so the static scan
    /// cannot place it in a family. When keys were family-ids, `SD006` fell back
    /// to its title while `ART-HL001` resolved to a family id; the two stopped
    /// merging and gov.uk's error tier doubled from 40 to 80. Canonicalizing
    /// through the title means the two schemes cannot disagree.
    /// Four near-identical heading-skip families existed, differing only by
    /// singular/plural and word order ("heading level skipped", "heading levels
    /// skipped", "heading level skip detected", "skipped heading level").
    /// `defect_signature` strips version and depth decorations but does not
    /// normalize word order, so the same defect was reported up to four times
    /// under four families.
    #[test]
    fn heading_skip_was_split_across_four_families() {
        let members = [
            "A11Y005", "HCOV002", "HEAD004", "HEADSC004", "HEADSKIP001", "HEADSKIP-V2001",
            "HEADSKIP-V5001", "HEADSKIP-V6121", "HHIER-V2003", "HHIER-V2004", "HORDER001",
            "HEADING-SKIPLEVEL",
        ];
        for code in members {
            assert_eq!(
                defect_key(code, "ignored: curated"),
                "heading level skipped",
                "{code} belongs to the single heading-skip family"
            );
        }
    }

    /// First-party plugin codes must name the family their title implies.
    ///
    /// The plugins originally reused built-in codes, shifted by one:
    /// `heading-structure` emitted `HEAD001` for "Multiple H1 headings" although
    /// `HEAD001` means "no headings found", and `meta-description-checker`
    /// emitted `META002` for "Meta description too short" although `META002`
    /// lives in the **title** family. Because `defect_key` resolves by code, the
    /// description finding was keyed as a title defect and merged into the
    /// built-in `TITLE001` — a silent false negative on any page with both a
    /// short title and a short description.
    #[test]
    fn first_party_plugin_codes_match_their_family() {
        for (code, expected_family) in [
            ("HEADING-MULTIH1", "multiple h1 headings"),
            ("HEADING-SKIPLEVEL", "heading level skipped"),
            ("HEADING-NONE", "no headings found"),
        ] {
            let key = defect_key(code, &defect_signature("irrelevant: code is curated"));
            assert_eq!(
                key, expected_family,
                "{code} must resolve to `{expected_family}`, not `{key}`"
            );
        }
    }

    /// Codes no family claims fall back to their normalized title, which is how
    /// the description-length codes stay out of the title family.
    #[test]
    fn description_codes_are_not_in_the_title_family() {
        for code in ["METADESC-MISSING", "METADESC-SHORT", "METADESC-LONG"] {
            assert!(
                !canonical_table().contains_key(code),
                "{code} must not claim a curated family; the title fallback keeps it distinct"
            );
        }
        let key = defect_key("METADESC-SHORT", &defect_signature("Meta description too short"));
        let title_key = defect_key("TITLE001", &defect_signature("Title too short"));
        assert_ne!(
            key, title_key,
            "a short description must not collapse into a short title"
        );
    }

    #[test]
    fn dynamic_title_sibling_merges_with_static_family() {
        let title = "article missing headline";
        assert_eq!(
            defect_key("SD006", title),
            defect_key("ART-HL001", title),
            "runtime-titled SD006 must merge with listed ART-HL001"
        );
        // And through the real registry, on a page with Article schema lacking
        // `headline`.
        use crate::analyzers::AnalyzerRegistry;
        use crate::parser::HtmlParser;
        use std::time::Duration;

        let html = r#"<html lang="en"><head><title>t</title>
        <script type="application/ld+json">
        {"@context":"https://schema.org","@type":"Article","name":"x","author":{"@type":"Person"}}
        </script></head><body><h1>t</h1></body></html>"#;
        let url = url::Url::parse("https://example.com/").expect("valid url");
        let page = HtmlParser::parse(html, &url);
        let headers: Vec<(String, String)> = Vec::new();
        let redirects: Vec<crate::RedirectHop> = Vec::new();
        let ctx = crate::analyzers::AnalysisContext {
            page: &page,
            body: Some(html),
            status_code: Some(200),
            headers: &headers,
            response_time: Some(Duration::from_millis(1)),
            redirect_chain: &redirects,
            robots_txt: None,
            user_agent: None,
            body_size: Some(html.len()),
            compressed_size: None,
            content_encoding: None,
            server: None,
            content_type: Some("text/html"),
            rendered: None,
        };
        let registry = AnalyzerRegistry::new(&crate::CrawlConfig::default());
        let findings = registry.analyze(&ctx);
        let headline: Vec<&str> = findings
            .iter()
            .filter(|f| f.title.eq_ignore_ascii_case("Article missing headline"))
            .map(|f| f.code.as_str())
            .collect();
        assert!(
            headline.len() <= 1,
            "the Article headline defect must report once, got {headline:?}"
        );
    }

    #[test]
    fn every_family_has_at_least_two_members() {
        for (title, codes) in CANONICAL_TITLES {
            assert!(codes.len() >= 2, "family {title} has a single member");
        }
    }

    #[test]
    fn every_canonical_title_is_non_empty_and_lowercase() {
        for (title, _) in CANONICAL_TITLES {
            assert!(!title.trim().is_empty(), "canonical title must not be blank");
        }
    }

    #[test]
    fn x_default_family_covers_all_six_codes() {
        let title = "missing x-default";
        let codes = ["HREF-V3001", "HREF-V4001", "HREF001", "HREFXD-V2001", "ISEO002", "HREFNET002"];
        let keys: Vec<String> = codes.iter().map(|c| defect_key(c, title)).collect();
        for k in &keys {
            assert_eq!(*k, keys[0], "all x-default codes must share a key");
        }
    }
}