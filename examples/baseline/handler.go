package main

func handler() {
	CheckAny(ctx, roles, "api:packages", "read")
	CheckAny(ctx, roles, "api:vouchers-suggestion", "create")
	CheckAny(ctx, roles, "api:vouchers", "create")
}
