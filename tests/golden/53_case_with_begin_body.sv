module m;
always_comb begin
case (sel)
2'd0: begin
a = 1;
b = 2;
end
default: a = 0;
endcase
end
endmodule
// expected -----
module m;
  always_comb begin
    case (sel)
      2'd0: begin
        a = 1;
        b = 2;
      end
      default: a = 0;
    endcase
  end
endmodule
