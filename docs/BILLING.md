# Billing Integration Guide

This guide covers integrating Stripe for billing in crawlkit.

## Setup

### 1. Create Stripe Account

1. Go to https://stripe.com
2. Create an account
3. Get API keys from dashboard

### 2. Configure Environment Variables

```bash
export STRIPE_SECRET_KEY=sk_test_...
export STRIPE_PUBLISHABLE_KEY=pk_test_...
export STRIPE_WEBHOOK_SECRET=whsec_...
```

### 3. Create Products and Prices

```bash
# Create product
curl https://api.stripe.com/v1/products \
  -u sk_test_...: \
  -d name="crawlkit-starter" \
  -d description="Starter plan for crawlkit"

# Create price
curl https://api.stripe.com/v1/prices \
  -u sk_test_...: \
  -d product=prod_... \
  -d unit_amount=2900 \
  -d currency=usd \
  -d "recurring[interval]"=month
```

## Pricing Tiers

| Tier | Price | Features |
|------|-------|----------|
| Free | $0/mo | 100 pages/crawl, 10 crawls/day |
| Starter | $29/mo | 1,000 pages/crawl, 100 crawls/day |
| Professional | $99/mo | 10,000 pages/crawl, unlimited crawls |
| Enterprise | Custom | Unlimited, SSO, RBAC, SLA |

## API Integration

### Create Subscription

```python
import stripe

stripe.api_key = "sk_test_..."

def create_subscription(customer_id, price_id):
    return stripe.Subscription.create(
        customer=customer_id,
        items=[{"price": price_id}],
        payment_behavior="default_incomplete",
        expand=["latest_invoice.payment_intent"],
    )
```

### Handle Webhooks

```python
from flask import Flask, request, jsonify

app = Flask(__name__)

@app.route("/webhook", methods=["POST"])
def stripe_webhook():
    payload = request.get_data()
    sig_header = request.headers.get("Stripe-Signature")

    event = stripe.Webhook.construct_event(
        payload, sig_header, os.environ["STRIPE_WEBHOOK_SECRET"]
    )

    if event["type"] == "invoice.payment_succeeded":
        # Update tenant subscription
        customer_id = event["data"]["object"]["customer"]
        update_tenant_subscription(customer_id, "active")

    elif event["type"] == "invoice.payment_failed":
        # Handle failed payment
        customer_id = event["data"]["object"]["customer"]
        handle_failed_payment(customer_id)

    return jsonify(success=True)
```

### Usage Tracking

```python
# Report usage to Stripe
def report_usage(subscription_item_id, quantity):
    stripe.SubscriptionItem.create_usage_record(
        subscription_item_id,
        quantity=quantity,
        timestamp=int(time.time()),
        action="increment",
    )
```

## Webhook Events

| Event | Description | Action |
|-------|-------------|--------|
| invoice.payment_succeeded | Payment successful | Activate subscription |
| invoice.payment_failed | Payment failed | Deactivate subscription |
| customer.subscription.deleted | Subscription canceled | Deactivate subscription |
| customer.subscription.updated | Subscription changed | Update plan |

## Testing

### Test Cards

| Number | Result |
|--------|--------|
| 4242 4242 4242 4242 | Success |
| 4000 0000 0000 0002 | Decline |
| 4000 0025 0000 3155 | 3D Secure |

### Test Webhooks

```bash
stripe trigger invoice.payment_succeeded
```
