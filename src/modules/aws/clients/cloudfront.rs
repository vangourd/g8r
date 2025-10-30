use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_cloudfront::Client as CloudFrontClient;
use aws_sdk_cloudfront::types::{
    DistributionConfig, Origins, Origin, CustomOriginConfig,
    OriginProtocolPolicy, DefaultCacheBehavior, ViewerProtocolPolicy,
    AllowedMethods, CachedMethods,
    TrustedSigners, ViewerCertificate, SslSupportMethod, MinimumProtocolVersion,
    Restrictions, GeoRestriction, GeoRestrictionType, Aliases,
    ForwardedValues, CookiePreference, Headers,
};
use serde_json::{json, Value as JsonValue};

use super::traits::CloudFrontOperations;

pub struct CloudFrontModule {
    client: CloudFrontClient,
}

impl CloudFrontModule {
    pub fn new(client: CloudFrontClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl CloudFrontOperations for CloudFrontModule {
    async fn create_distribution(&self, config: JsonValue) -> Result<(String, String)> {
        let origin_domain = config["origin_domain"].as_str()
            .context("Missing origin_domain")?;
        let origin_id = config["origin_id"].as_str()
            .context("Missing origin_id")?;
        let aliases = config["aliases"].as_array()
            .context("Missing aliases")?;
        let certificate_arn = config["certificate_arn"].as_str()
            .context("Missing certificate_arn")?;
        let caller_ref = format!("g8r-{}", chrono::Utc::now().timestamp());

        let custom_origin = CustomOriginConfig::builder()
            .http_port(80)
            .https_port(443)
            .origin_protocol_policy(OriginProtocolPolicy::HttpOnly)
            .build()
            .context("Failed to build custom origin config")?;

        let origin = Origin::builder()
            .id(origin_id)
            .domain_name(origin_domain)
            .custom_origin_config(custom_origin)
            .build()
            .context("Failed to build origin")?;

        let origins = Origins::builder()
            .items(origin)
            .quantity(1)
            .build()
            .context("Failed to build origins")?;

        let allowed_methods_list = AllowedMethods::builder()
            .items(aws_sdk_cloudfront::types::Method::Get)
            .items(aws_sdk_cloudfront::types::Method::Head)
            .quantity(2)
            .cached_methods(
                CachedMethods::builder()
                    .items(aws_sdk_cloudfront::types::Method::Get)
                    .items(aws_sdk_cloudfront::types::Method::Head)
                    .quantity(2)
                    .build()
                    .context("Failed to build cached methods")?
            )
            .build()
            .context("Failed to build allowed methods")?;

        let trusted_signers = TrustedSigners::builder()
            .enabled(false)
            .quantity(0)
            .build()
            .context("Failed to build trusted signers")?;

        let cookie_preference = CookiePreference::builder()
            .forward(aws_sdk_cloudfront::types::ItemSelection::None)
            .build()
            .context("Failed to build cookie preference")?;

        let headers = Headers::builder()
            .quantity(0)
            .build()
            .context("Failed to build headers")?;

        let forwarded_values = ForwardedValues::builder()
            .query_string(false)
            .cookies(cookie_preference)
            .headers(headers)
            .build()
            .context("Failed to build forwarded values")?;

        let default_cache_behavior = DefaultCacheBehavior::builder()
            .target_origin_id(origin_id)
            .viewer_protocol_policy(ViewerProtocolPolicy::RedirectToHttps)
            .allowed_methods(allowed_methods_list)
            .trusted_signers(trusted_signers)
            .compress(true)
            .min_ttl(0)
            .default_ttl(86400)
            .max_ttl(31536000)
            .forwarded_values(forwarded_values)
            .build()
            .context("Failed to build default cache behavior")?;

        let viewer_certificate = ViewerCertificate::builder()
            .acm_certificate_arn(certificate_arn)
            .ssl_support_method(SslSupportMethod::SniOnly)
            .minimum_protocol_version(MinimumProtocolVersion::TlSv122021)
            .build();

        let geo_restriction = GeoRestriction::builder()
            .restriction_type(GeoRestrictionType::None)
            .quantity(0)
            .build()
            .context("Failed to build geo restriction")?;

        let restrictions = Restrictions::builder()
            .geo_restriction(geo_restriction)
            .build();

        let mut aliases_builder = Aliases::builder().quantity(aliases.len() as i32);
        for alias in aliases {
            if let Some(alias_str) = alias.as_str() {
                aliases_builder = aliases_builder.items(alias_str);
            }
        }
        let aliases_obj = aliases_builder.build().context("Failed to build aliases")?;

        let dist_config = DistributionConfig::builder()
            .caller_reference(caller_ref)
            .origins(origins)
            .default_cache_behavior(default_cache_behavior)
            .comment("Created by g8r")
            .enabled(true)
            .is_ipv6_enabled(true)
            .default_root_object("index.html")
            .aliases(aliases_obj)
            .viewer_certificate(viewer_certificate)
            .restrictions(restrictions)
            .build()
            .context("Failed to build distribution config")?;

        let result = self.client
            .create_distribution()
            .distribution_config(dist_config)
            .send()
            .await
            .context("Failed to create CloudFront distribution")?;

        let distribution = result.distribution().context("No distribution in response")?;
        let id = distribution.id().to_string();
        let domain_name = distribution.domain_name().to_string();

        Ok((id, domain_name))
    }

    async fn get_distribution(&self, id: &str) -> Result<Option<JsonValue>> {
        match self.client.get_distribution().id(id).send().await {
            Ok(result) => {
                let _dist = result.distribution().context("No distribution in response")?;
                Ok(Some(json!({"status": "ok"})))
            }
            Err(e) if e.to_string().contains("NoSuchDistribution") => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to get distribution: {}", e)),
        }
    }

    async fn disable_distribution(&self, id: &str) -> Result<()> {
        let get_result = self.client.get_distribution_config()
            .id(id)
            .send()
            .await
            .context("Failed to get distribution config")?;
        
        let etag = get_result.e_tag()
            .context("No ETag in response")?
            .to_string();
        
        let config = get_result.distribution_config()
            .context("No distribution config in response")?
            .clone();
        
        let disabled_config = DistributionConfig::builder()
            .caller_reference(config.caller_reference())
            .set_origins(config.origins().cloned())
            .set_default_cache_behavior(config.default_cache_behavior().cloned())
            .comment(config.comment())
            .set_aliases(config.aliases().cloned())
            .set_viewer_certificate(config.viewer_certificate().cloned())
            .set_restrictions(config.restrictions().cloned())
            .set_default_root_object(config.default_root_object().map(|s| s.to_string()))
            .set_is_ipv6_enabled(config.is_ipv6_enabled())
            .enabled(false)
            .build()
            .context("Failed to build disabled distribution config")?;
        
        self.client.update_distribution()
            .id(id)
            .distribution_config(disabled_config)
            .if_match(etag)
            .send()
            .await
            .context("Failed to disable distribution")?;
        
        Ok(())
    }

    async fn delete_distribution(&self, id: &str) -> Result<()> {
        let get_result = self.client.get_distribution()
            .id(id)
            .send()
            .await
            .context("Failed to get distribution")?;
        
        let etag = get_result.e_tag()
            .context("No ETag in response")?
            .to_string();
        
        self.client.delete_distribution()
            .id(id)
            .if_match(etag)
            .send()
            .await
            .context("Failed to delete distribution")?;
        
        Ok(())
    }
}
