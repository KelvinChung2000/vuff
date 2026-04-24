module m;
  logic q;
  always @ ( posedge clk or negedge rst_n ) begin
    q <= d;
  end
endmodule
// expected -----
module m;
  logic q;
  always @(posedge clk or negedge rst_n) begin
    q <= d;
  end
endmodule
