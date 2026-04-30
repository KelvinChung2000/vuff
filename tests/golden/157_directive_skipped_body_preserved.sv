module m;
`ifdef DEBUG
  // Diagnostic block — keep me even when DEBUG is undefined.
  always @(posedge clk) begin
    $display("debug %d", x);
  end
`endif
  logic active_only;
endmodule
// expected -----
module m;
  `ifdef DEBUG
    // Diagnostic block — keep me even when DEBUG is undefined.
    always @(posedge clk) begin
      $display("debug %d", x);
    end
  `endif
  logic active_only;
endmodule
