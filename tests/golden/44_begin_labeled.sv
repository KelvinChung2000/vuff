module m;
initial begin : my_label
x = 1;
end : my_label
endmodule
// expected -----
module m;
  initial begin : my_label
    x = 1;
  end : my_label
endmodule
