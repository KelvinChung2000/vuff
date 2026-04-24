module top(input wire clk);
(*keep_hierarchy="yes"*)
sub u_sub(.clk(clk));
endmodule
// expected -----
module top (
  input wire clk
);
  (* keep_hierarchy = "yes" *)
  sub u_sub (.clk(clk));
endmodule
