# =============================================================================
# GitHub Actions OIDC Configuration for VERIDACTUS
# Allows secure authentication with cloud providers without storing secrets
# =============================================================================

# This workflow file is kept for documentation purposes.
# OIDC is configured through repository settings:
# Settings → Security → Secrets and variables → Actions → OpenID Connect

# For AWS:
# - Add IAM role with trust policy for:
#   resource: "arn:aws:iam::ACCOUNT:role/veridactus-deploy"
#   condition: StringEquals:
#     token.actions.githubusercontent.com:sub: "repo:veridactus/veridactus:*"

# For GCP:
# - Add workload identity provider with:
#   resource: "//iam.googleapis.com/projects/PROJECT/workloadIdentityPools/POOL/providers/PROVIDER"
#   condition:
#     condition_type: "_attribute.repository"
#     attribute_value: "veridactus/veridactus"
