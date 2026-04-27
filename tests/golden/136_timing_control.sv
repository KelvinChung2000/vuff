module m;
  initial begin
    @(posedge clk) x = 1;
    #5 y = 1;
    #(5+3) z = 1;
    ##2 q = w;
  end
endmodule
// expected -----
module m;
  initial begin
    @(posedge clk) x = 1;
    #5 y = 1;
    #(5 + 3) z = 1;
    ##2 q = w;
  end
endmodule
