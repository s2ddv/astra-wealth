use application::fx::{FxError, FxFuture, FxProvider, FxQuote};
use chrono::{Duration, NaiveDate};
use reqwest::Client;
use serde::Deserialize;

pub struct BcbPtaxProvider {
    client: Client,
    base: String,
}
impl BcbPtaxProvider {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            base: "https://olinda.bcb.gov.br/olinda/servico/PTAX/versao/v1/odata".into(),
        })
    }
}
#[derive(Deserialize)]
struct Payload {
    value: Vec<Row>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    cotacao_venda: serde_json::Number,
    data_hora_cotacao: String,
    tipo_boletim: String,
}
impl FxProvider for BcbPtaxProvider {
    fn closing_on_or_before(&self, date: NaiveDate) -> FxFuture<'_> {
        Box::pin(async move {
            let start = (date - Duration::days(14)).format("'%m-%d-%Y'").to_string();
            let end = date.format("'%m-%d-%Y'").to_string();
            let mut url = url::Url::parse(&format!("{}/CotacaoMoedaPeriodo(moeda=@moeda,dataInicial=@dataInicial,dataFinalCotacao=@dataFinalCotacao)", self.base)).map_err(|_| FxError::Unavailable)?;
            url.query_pairs_mut().extend_pairs([
                ("@moeda", "'USD'"),
                ("@dataInicial", &start),
                ("@dataFinalCotacao", &end),
                ("$filter", "tipoBoletim eq 'Fechamento'"),
                ("$orderby", "dataHoraCotacao desc"),
                ("$top", "1"),
                ("$format", "json"),
                ("$select", "cotacaoVenda,dataHoraCotacao,tipoBoletim"),
            ]);
            // Olinda treats '+' literally in OData expressions; spaces must be %20.
            let query = url.query().unwrap_or_default().replace('+', "%20");
            url.set_query(Some(&query));
            let mut response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|_| FxError::Unavailable)?
                .error_for_status()
                .map_err(|_| FxError::Unavailable)?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| FxError::Unavailable)? {
                if bytes.len() + chunk.len() > 65536 {
                    return Err(FxError::Unavailable);
                }
                bytes.extend_from_slice(&chunk);
            }
            parse(&bytes, date)
        })
    }
}
fn parse(bytes: &[u8], date: NaiveDate) -> Result<Option<FxQuote>, FxError> {
    let payload: Payload = serde_json::from_slice(bytes).map_err(|_| FxError::Unavailable)?;
    if payload.value.len() > 1 {
        return Err(FxError::Unavailable);
    }
    let Some(row) = payload.value.into_iter().next() else {
        return Ok(None);
    };
    let quote = FxQuote {
        rate: row.cotacao_venda.to_string(),
        effective_date: row
            .data_hora_cotacao
            .get(..10)
            .ok_or(FxError::Unavailable)?
            .into(),
        published_at: row.data_hora_cotacao,
    };
    if row.tipo_boletim != "Fechamento" || !application::fx::valid_quote(&quote, date) {
        return Err(FxError::Unavailable);
    }
    Ok(Some(quote))
}

#[cfg(test)]
mod tests;
